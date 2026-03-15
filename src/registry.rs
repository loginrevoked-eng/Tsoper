use winreg::enums::*;
use winreg::RegKey;
use crate::error::{Result, DowmanError};

pub fn add_to_registry(keypath: &str, keyname: &str, keyvalue: &str) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (dowman_key, _) = hkcu.create_subkey(keypath)
        .map_err(|e| DowmanError::Registry(e))?;
        
    dowman_key.set_value(keyname, &keyvalue)
        .map_err(|e| DowmanError::Registry(e))?;
        
    Ok(())
}
pub fn get_from_registry(path: &str, key: &str) -> std::io::Result<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let subkey = hkcu.open_subkey(path)?;
    let val: String = subkey.get_value(key)?;
    Ok(val)
}