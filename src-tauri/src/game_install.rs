use serde::Serialize;
use steamlocate::SteamDir;

const GAME_APP_ID: u32 = 1_761_390;

#[derive(Debug, Serialize)]
pub struct GameInstallation {
    pub game_dir: String,
    pub mods_dir: Option<String>,
}

pub fn detect_game_installation() -> Result<Option<GameInstallation>, String> {
    let steam = match steamlocate::locate() {
        Ok(steam) => steam,
        Err(steamlocate::Error::FailedLocate(_) | steamlocate::Error::InvalidSteamDir(_)) => {
            return Ok(None);
        }
        Err(error) => return Err(format!("读取 Steam 安装信息失败：{error}")),
    };
    find_game_in_steam(&steam)
}

fn find_game_in_steam(steam: &SteamDir) -> Result<Option<GameInstallation>, String> {
    let libraries = steam
        .libraries()
        .map_err(|error| format!("读取 Steam 游戏库失败：{error}"))?;

    // 外接盘离线、库不可读或单个清单损坏时，仍继续检查其他已登记的库。
    for library in libraries.flatten() {
        let Some(Ok(app)) = library.app(GAME_APP_ID) else {
            continue;
        };
        if app.app_id != GAME_APP_ID {
            continue;
        }
        let game_dir = library.resolve_app_dir(&app);
        if !game_dir.join("DivaMegaMix.exe").is_file() {
            continue;
        }
        let mods_dir = game_dir.join("mods");
        return Ok(Some(GameInstallation {
            game_dir: game_dir.to_string_lossy().into_owned(),
            mods_dir: mods_dir
                .is_dir()
                .then(|| mods_dir.to_string_lossy().into_owned()),
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct SteamFixture {
        root: PathBuf,
        steam_path: PathBuf,
    }

    impl SteamFixture {
        fn new() -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "diva_game_install_{}_{}",
                std::process::id(),
                unique
            ));
            fs::create_dir(&root).unwrap();
            let steam_path = root.join("Steam");
            fs::create_dir_all(steam_path.join("steamapps")).unwrap();
            Self { root, steam_path }
        }

        fn library(&self, name: &str) -> PathBuf {
            let path = self.root.join(name);
            fs::create_dir_all(path.join("steamapps")).unwrap();
            path
        }

        fn steam(&self, libraries: &[&Path]) -> SteamDir {
            let mut vdf = String::from("\"libraryfolders\"\n{\n");
            for (index, path) in libraries.iter().enumerate() {
                let path = path
                    .to_str()
                    .unwrap()
                    .replace('\\', "\\\\")
                    .replace('"', "\\\"");
                vdf.push_str(&format!(
                    "\"{index}\" {{ \"path\" \"{path}\" }}\n"
                ));
            }
            vdf.push_str("}\n");
            fs::write(self.steam_path.join("steamapps/libraryfolders.vdf"), vdf).unwrap();
            SteamDir::from_dir(&self.steam_path).unwrap()
        }

        fn install(&self, library: &Path, name: &str) -> PathBuf {
            fs::write(
                library.join(format!("steamapps/appmanifest_{GAME_APP_ID}.acf")),
                format!(
                    "\"AppState\" {{ \"appid\" \"{GAME_APP_ID}\" \"installdir\" \"{name}\" }}"
                ),
            )
            .unwrap();
            let game_dir = library.join("steamapps").join("common").join(name);
            fs::create_dir_all(&game_dir).unwrap();
            game_dir
        }
    }

    impl Drop for SteamFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn finds_game_and_mods_in_non_default_library_with_unicode_and_spaces() {
        let fixture = SteamFixture::new();
        let library = fixture.library("另一游戏库 Steam Library");
        let game_dir = fixture.install(&library, "初音未来 Project DIVA Mega Mix Plus");
        fs::write(game_dir.join("DivaMegaMix.exe"), []).unwrap();
        fs::create_dir(game_dir.join("mods")).unwrap();
        let steam = fixture.steam(&[&fixture.steam_path, &library]);

        let found = find_game_in_steam(&steam).unwrap().unwrap();

        assert_eq!(Path::new(&found.game_dir), game_dir);
        assert_eq!(
            Path::new(found.mods_dir.as_deref().unwrap()),
            game_dir.join("mods")
        );
    }

    #[test]
    fn rejects_manifest_without_executable_file() {
        let fixture = SteamFixture::new();
        let game_dir = fixture.install(&fixture.steam_path, "Project DIVA");
        let steam = fixture.steam(&[&fixture.steam_path]);

        assert!(find_game_in_steam(&steam).unwrap().is_none());
        fs::create_dir(game_dir.join("DivaMegaMix.exe")).unwrap();
        assert!(find_game_in_steam(&steam).unwrap().is_none());
    }

    #[test]
    fn returns_game_without_creating_mods_directory() {
        let fixture = SteamFixture::new();
        let game_dir = fixture.install(&fixture.steam_path, "Project DIVA");
        fs::write(game_dir.join("DivaMegaMix.exe"), []).unwrap();
        let steam = fixture.steam(&[&fixture.steam_path]);

        let found = find_game_in_steam(&steam).unwrap().unwrap();

        assert_eq!(Path::new(&found.game_dir), game_dir);
        assert!(found.mods_dir.is_none());
        assert!(!game_dir.join("mods").exists());
        fs::write(game_dir.join("mods"), []).unwrap();
        assert!(find_game_in_steam(&steam)
            .unwrap()
            .unwrap()
            .mods_dir
            .is_none());
    }

    #[test]
    fn skips_unavailable_libraries_and_broken_manifests() {
        let fixture = SteamFixture::new();
        let missing_library = fixture.root.join("离线库");
        let broken_library = fixture.library("损坏清单库");
        fs::write(
            broken_library.join(format!("steamapps/appmanifest_{GAME_APP_ID}.acf")),
            "损坏的清单",
        )
        .unwrap();
        let unreadable_library = fixture.library("不可读取清单库");
        fs::create_dir(
            unreadable_library.join(format!("steamapps/appmanifest_{GAME_APP_ID}.acf")),
        )
        .unwrap();
        let valid_library = fixture.library("有效库");
        let game_dir = fixture.install(&valid_library, "Project DIVA");
        fs::write(game_dir.join("DivaMegaMix.exe"), []).unwrap();
        let steam = fixture.steam(&[
            &missing_library,
            &broken_library,
            &unreadable_library,
            &valid_library,
        ]);

        let found = find_game_in_steam(&steam).unwrap().unwrap();

        assert_eq!(Path::new(&found.game_dir), game_dir);
        assert!(!missing_library.exists());
    }

    #[test]
    fn reports_unparseable_library_list() {
        let fixture = SteamFixture::new();
        let steam = fixture.steam(&[&fixture.steam_path]);
        fs::write(
            fixture.steam_path.join("steamapps/libraryfolders.vdf"),
            "损坏的游戏库列表",
        )
        .unwrap();

        assert!(find_game_in_steam(&steam).is_err());
    }
}
