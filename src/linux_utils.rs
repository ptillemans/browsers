use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use glib::prelude::AppInfoExt;
use glib::AppInfo;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme};

use crate::{InstalledBrowser, SupportedAppRepository};

const XDG_NAME: &'static str = "software.Browsers";

pub struct OsHelper {
    app_repository: SupportedAppRepository,
    icon_theme: Arc<Mutex<IconTheme>>,
}

unsafe impl Send for OsHelper {}

impl OsHelper {
    // must be initialized in main thread (because of gtk requirements)
    pub fn new() -> OsHelper {
        let _result = gtk::init();
        let icon_theme = gtk::IconTheme::default().unwrap();
        let app_repository = SupportedAppRepository::new();
        Self {
            app_repository: app_repository,
            icon_theme: Arc::new(Mutex::new(icon_theme)),
            // unsandboxed_home_dir: unsandboxed_home_dir().unwrap(), probably needed if snap pkg
        }
    }

    pub fn get_app_repository(&self) -> &SupportedAppRepository {
        return &self.app_repository;
    }

    pub fn get_installed_browsers(&self) -> Vec<crate::InstalledBrowser> {
        let mut browsers: Vec<crate::InstalledBrowser> = Vec::new();

        /*let app_infos = AppInfo::all_for_type("x-scheme-handler/spotify");
        for app_info in app_infos {
            let browser_maybe = parser.parse_app_info(app_info);
            if browser_maybe.is_some() {
                browsers.push(browser_maybe.unwrap());
            }
        }*/

        let app_infos = AppInfo::all_for_type("x-scheme-handler/https");
        for app_info in app_infos {
            let browser_maybe = self.to_installed_browser(app_info);
            if browser_maybe.is_some() {
                browsers.push(browser_maybe.unwrap());
            }
        }

        return browsers;
    }

    fn to_installed_browser(&self, app_info: AppInfo) -> Option<InstalledBrowser> {
        let option = app_info.commandline();
        // uses %u or %U, see https://specifications.freedesktop.org/desktop-entry-spec/latest/ar01s07.html
        // need to use command(), because executable() is not fine, as snap apps just have "env" there
        let command_with_field_codes = option.unwrap();
        let command_str = command_with_field_codes.to_str().unwrap();
        let command_str = str::replace(command_str, " %u", "");
        let mut command_str = str::replace(command_str.as_str(), " %U", "");

        // handle snap packages in a very naive way; TODO: VERY FRAGILE

        // "env BAMF_DESKTOP_FILE_HINT=/var/lib/snapd/desktop/applications/firefox_firefox.desktop /snap/bin/firefox"
        // to "/snap/bin/firefox"

        if command_str.starts_with("env BAMF_DESKTOP_FILE_HINT=") {
            let command_components: Vec<&str> = command_str.split(' ').collect();
            let actual_cmd_maybe = command_components.get(2);
            let actual_cmd = actual_cmd_maybe.unwrap();
            let actual_cmd_str = actual_cmd.to_string();
            command_str = actual_cmd_str;
            //let actual_command = command_components.get(2);
        }
        let executable_path = Path::new(command_str.as_str());

        // TODO: get correct path for firefox snap, which one is actually used to calculate installation id in profiles.ini
        // let command_dir = executable_path.parent();
        // let binary_dir = command_dir.and_then(|p| p.to_str()).unwrap_or("").to_string();

        // env BAMF_DESKTOP_FILE_HINT=/var/lib/snapd/desktop/applications/firefox_firefox.desktop
        //

        // let executable_path = app_info.executable(); // wrong for snaps (env)

        let name = app_info.name().to_string();
        /*let icon_maybe = app_info.icon();
        let icon: Icon = icon_maybe.unwrap();*/

        let id_maybe = app_info.id();
        if id_maybe.is_none() {
            println!("no id found for {}", name);
            return None;
        }

        let id_gstring = id_maybe.unwrap();
        let id = id_gstring.as_str().to_string();
        // "google-chrome-beta.desktop"

        if id == "software.Browsers.desktop" {
            // this is us, skip
            return None;
        }

        let supported_app = self.app_repository.get_or_generate(id.as_str());

        let string1 = app_info.display_name();
        let display_name = string1.as_str();
        let _string = app_info.to_string();
        //println!("app_info: {}", id);

        let icon_maybe = app_info.icon();

        let icon = icon_maybe.unwrap();
        //let icon_str = IconExt::to_string(&icon);
        //let icon_gstr = icon_str.unwrap();
        //let string2 = icon_gstr.to_string();

        let icon_theme = Arc::clone(&self.icon_theme);
        let icon_theme2 = icon_theme.lock().unwrap();

        let icon_info = icon_theme2
            .lookup_by_gicon(&icon, 48, IconLookupFlags::USE_BUILTIN)
            .unwrap();

        // to support scaled resolutions
        //let icon_info = icon_theme.lookup_by_gicon_for_scale(&icon, 128, 1,IconLookupFlags::USE_BUILTIN).unwrap();

        // or load_icon() to get PixBuf
        let icon_filepath = icon_info.filename().unwrap();

        let icon_path_str = icon_filepath.as_path().to_str().unwrap().to_string();
        println!("icon: {}", icon_path_str);

        // https://lazka.github.io/pgi-docs/Gio-2.0/interfaces/Icon.html#Gio.Icon.to_string
        // either file path or themed icon name

        let profiles = supported_app.find_profiles(executable_path.clone());

        let browser = crate::InstalledBrowser {
            executable_path: executable_path.to_str().unwrap().to_string(),
            display_name: display_name.to_string(),
            bundle: supported_app.get_app_id().to_string(),
            user_dir: supported_app.get_app_config_dir_absolute().to_string(),
            icon_path: icon_path_str.clone(),
            profiles: profiles,
        };
        return Some(browser);
    }
}

// $HOME/.local/share/software.Browsers
pub fn get_this_app_config_root_dir() -> PathBuf {
    return get_this_app_xdg_data_dir();
}

// $HOME/.local/share/software.Browsers
fn get_this_app_xdg_data_dir() -> PathBuf {
    // $XDG_STATE_HOME or $HOME/.local/state
    return dirs::data_dir().unwrap().join(XDG_NAME);
}

// $HOME/.cache/software.Browsers
pub fn get_this_app_cache_root_dir() -> PathBuf {
    // $XDG_CACHE_HOME or $HOME/.cache
    return dirs::cache_dir().unwrap().join(XDG_NAME);
}

// $HOME/.local/state/software.Browsers/logs
pub fn get_this_app_logs_root_dir() -> PathBuf {
    return get_this_app_xdg_state_dir().join("logs");
}

// $HOME/.local/state/software.Browsers
fn get_this_app_xdg_state_dir() -> PathBuf {
    // $XDG_STATE_HOME or $HOME/.local/state
    let state_dir = dirs::state_dir().unwrap();
    return state_dir.join(XDG_NAME);
}

pub fn linux_get_unsandboxed_config_dir() -> PathBuf {
    // TODO: escape sandbox if in snap/flatpak
    return dirs::config_dir().unwrap();
}

pub fn linux_get_unsandboxed_home_dir() -> PathBuf {
    // TODO: escape sandbox if in snap/flatpak
    return dirs::home_dir().unwrap();
}