use crate::net::HTTP;
use crate::store::{self, Account, R, S};
use serde_json::{json, Value};

pub fn offline_uuid(name: &str) -> String {
    let mut h = md5::compute(format!("OfflinePlayer:{name}")).0;
    h[6] = (h[6] & 0x0f) | 0x30; h[8] = (h[8] & 0x3f) | 0x80;
    hex::encode(h)
}
fn accounts() -> Vec<Account> { store::load("accounts.json") }
fn save(v: &Vec<Account>) -> R<()> { store::save("accounts.json", v) }
fn entry(id: &str) -> R<keyring::Entry> { keyring::Entry::new("thm-launcher", id).s() }

#[tauri::command]
pub fn list_accounts() -> Vec<Account> { accounts() }

#[tauri::command]
pub fn add_offline(username: String) -> R<Account> {
    let u = username.trim().to_string();
    if u.is_empty() || u.len() > 16 || !u.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Username must be 1–16 characters: letters, numbers, underscore".into());
    }
    let mut v = accounts();
    if v.iter().any(|a| a.kind == "offline" && a.username.eq_ignore_ascii_case(&u)) { return Err("That offline profile already exists".into()); }
    let a = Account { id: uuid::Uuid::new_v4().to_string(), kind: "offline".into(), username: u.clone(), uuid: offline_uuid(&u), active: v.is_empty() };
    v.push(a.clone()); save(&v)?; Ok(a)
}
#[tauri::command]
pub fn remove_account(id: String) -> R<()> {
    let mut v = accounts(); v.retain(|a| a.id != id);
    if !v.is_empty() && !v.iter().any(|a| a.active) { v[0].active = true; }
    entry(&id).and_then(|e| e.delete_credential().s()).ok();
    save(&v)
}
#[tauri::command]
pub fn set_active_account(id: String) -> R<()> {
    let mut v = accounts();
    if !v.iter().any(|a| a.id == id) { return Err("account not found".into()); }
    for a in v.iter_mut() { a.active = a.id == id; }
    save(&v)
}

const LIVE_ID: &str = "00000000402b5328"; // Mojang's own launcher client ID: no per-user Azure setup
const LIVE_SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";
const TENANT: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0";

struct Ep { cid: String, dev: String, tok: String, scope: String, prefix: &'static str }
/// Default is the built-in flow. A custom Azure client ID in Settings (optional, advanced) overrides it.
fn ep() -> Ep {
    let c = store::settings().ms_client_id; let c = c.trim().to_string();
    if c.is_empty() {
        Ep { cid: LIVE_ID.into(), dev: "https://login.live.com/oauth20_connect.srf".into(), tok: "https://login.live.com/oauth20_token.srf".into(), scope: LIVE_SCOPE.into(), prefix: "t=" }
    } else {
        Ep { cid: c, dev: format!("{TENANT}/devicecode"), tok: format!("{TENANT}/token"), scope: "XboxLive.signin offline_access".into(), prefix: "d=" }
    }
}

#[tauri::command]
pub async fn ms_begin() -> R<Value> {
    let e = ep();
    let r: Value = HTTP.post(&e.dev).form(&[("client_id", e.cid.as_str()), ("scope", e.scope.as_str()), ("response_type", "device_code")])
        .send().await.s()?.json().await.s()?;
    if r.get("device_code").is_none() { return Err(r["error_description"].as_str().unwrap_or("Could not start Microsoft login").into()); }
    Ok(r)
}

/// Returns Ok(None) while the user hasn't finished signing in.
#[tauri::command]
pub async fn ms_poll(device_code: String) -> R<Option<Account>> {
    let e = ep();
    let r: Value = HTTP.post(&e.tok).form(&[("grant_type", "urn:ietf:params:oauth:grant-type:device_code"), ("client_id", e.cid.as_str()), ("device_code", device_code.as_str())])
        .send().await.s()?.json().await.s()?;
    match r["error"].as_str() {
        Some("authorization_pending") | Some("slow_down") => return Ok(None),
        Some(_) => return Err(r["error_description"].as_str().unwrap_or("Login failed").lines().next().unwrap_or("Login failed").into()),
        None => {}
    }
    let refresh = r["refresh_token"].as_str().ok_or("no refresh token")?.to_string();
    let (_tok, uuid, name) = mc_chain(r["access_token"].as_str().ok_or("no access token")?, e.prefix).await?;
    let mut v = accounts();
    let id = v.iter().find(|a| a.kind == "microsoft" && a.uuid == uuid).map(|a| a.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    entry(&id)?.set_password(&refresh).s()?;
    v.retain(|a| a.id != id);
    let a = Account { id, kind: "microsoft".into(), username: name, uuid, active: v.is_empty() };
    v.push(a.clone()); save(&v)?; Ok(Some(a))
}

async fn post_json(url: &str, body: Value) -> R<Value> {
    let r = HTTP.post(url).json(&body).header("Accept", "application/json").send().await.s()?;
    let st = r.status(); let j: Value = r.json().await.unwrap_or(Value::Null);
    if !st.is_success() {
        if let Some(x) = j["XErr"].as_u64() {
            return Err(match x { 2148916233 => "This Microsoft account has no Xbox profile. Sign in at xbox.com once to create one.".into(),
                2148916238 => "This is a child account; it must be added to a Family by an adult.".into(), _ => format!("Xbox error {x}") });
        }
        return Err(format!("{url} failed ({st})"));
    }
    Ok(j)
}

/// Xbox Live -> XSTS -> Minecraft services -> ownership check -> profile.
async fn mc_chain(ms_access: &str, prefix: &str) -> R<(String, String, String)> {
    let xbl = post_json("https://user.auth.xboxlive.com/user/authenticate", json!({"Properties":{"AuthMethod":"RPS","SiteName":"user.auth.xboxlive.com","RpsTicket":format!("{prefix}{ms_access}")},"RelyingParty":"http://auth.xboxlive.com","TokenType":"JWT"})).await?;
    let (xt, uhs) = (xbl["Token"].as_str().ok_or("XBL token missing")?, xbl.pointer("/DisplayClaims/xui/0/uhs").and_then(|v| v.as_str()).ok_or("XBL uhs missing")?.to_string());
    let xsts = post_json("https://xsts.auth.xboxlive.com/xsts/authorize", json!({"Properties":{"SandboxId":"RETAIL","UserTokens":[xt]},"RelyingParty":"rp://api.minecraftservices.com/","TokenType":"JWT"})).await?;
    let mc = post_json("https://api.minecraftservices.com/authentication/login_with_xbox", json!({"identityToken":format!("XBL3.0 x={uhs};{}", xsts["Token"].as_str().ok_or("XSTS token missing")?)})).await?;
    let token = mc["access_token"].as_str().ok_or("Minecraft token missing")?.to_string();
    let ent: Value = HTTP.get("https://api.minecraftservices.com/entitlements/mcstore").bearer_auth(&token).send().await.s()?.json().await.s()?;
    if ent["items"].as_array().map(|a| a.is_empty()).unwrap_or(true) { return Err("This account does not own Minecraft: Java Edition.".into()); }
    let p = HTTP.get("https://api.minecraftservices.com/minecraft/profile").bearer_auth(&token).send().await.s()?;
    if !p.status().is_success() { return Err("No Minecraft profile on this account (create one at minecraft.net).".into()); }
    let p: Value = p.json().await.s()?;
    Ok((token, p["id"].as_str().unwrap_or("").into(), p["name"].as_str().unwrap_or("").into()))
}

/// Produces a launch session. Microsoft tokens are refreshed each launch and kept only in memory.
pub async fn session(a: &Account) -> R<crate::mc::Session> {
    if a.kind == "offline" { return Ok(crate::mc::Session { name: a.username.clone(), uuid: a.uuid.clone(), token: "0".into(), user_type: "legacy".into() }); }
    let refresh = entry(&a.id)?.get_password().map_err(|_| "Saved login missing — please sign in again.".to_string())?;
    let e = ep();
    let r: Value = HTTP.post(&e.tok).form(&[("grant_type", "refresh_token"), ("client_id", e.cid.as_str()), ("refresh_token", refresh.as_str()), ("scope", e.scope.as_str())])
        .send().await.s()?.json().await.s()?;
    let acc = r["access_token"].as_str().ok_or("Session expired — please sign in again.")?;
    if let Some(nr) = r["refresh_token"].as_str() { entry(&a.id)?.set_password(nr).s()?; }
    let (token, uuid, name) = mc_chain(acc, e.prefix).await?;
    Ok(crate::mc::Session { name, uuid, token, user_type: "msa".into() })
}
pub fn active() -> R<Account> { accounts().into_iter().find(|a| a.active).ok_or_else(|| "No active account — add one in Accounts".to_string()) }
