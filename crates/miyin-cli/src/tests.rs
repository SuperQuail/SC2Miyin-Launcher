//! CLI 的单元测试。
//!
//! 建在 main.rs 里面的 `mod tests` 会让那份文件越过仓库的 500 行门禁 ——
//! 挪出来和 miyin-core 的做法一致（见 AGENTS.md §6）。

use super::*;

mod tests {
    use super::*;

    /// 造一个测试 zip（游戏版权内容一律不许进测试，所以自己写）。
    fn build_zip(dir: &Path, name: &str, files: &[(&str, &[u8])]) -> PathBuf {
        let path = dir.join(name);
        let file = std::fs::File::create(&path).expect("create zip");
        let mut writer = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (entry, content) in files {
            writer.start_file(*entry, options).expect("start file");
            writer.write_all(content).expect("write");
        }
        writer.finish().expect("finish");
        path
    }

    fn payload(target: PayloadTarget) -> Payload {
        Payload {
            source: "x".to_string(),
            expanded: false,
            is_mod: false,
            target,
        }
    }

    /// 一个声明了归属的好包：必须能算出落点，槽位就是包内声明的那一个。
    #[test]
    fn inspects_a_well_formed_ccm_package() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "reborn.zip",
            &[
                (
                    "Reborn/metadata.txt",
                    b"title=Reborn\nauthor=Someone\ncampaign=LOTV\nversion=1.0\n".as_slice(),
                ),
                ("Reborn/maps/paiur01.SC2Map", b"stub".as_slice()),
            ],
        );

        let inspection = inspect_package(&archive).expect("预检应成功");
        assert!(inspection.installable, "问题：{:?}", inspection.issues);
        assert_eq!(inspection.suggested_slot.as_deref(), Some("lotv"));
        assert_eq!(inspection.campaign_type, CampaignType::Lotv);
        // 没有走证据链 —— 元数据已经说清楚了
        assert!(inspection.identification.is_none());
    }

    /// 认不出归属时 campaign_type 是 Other —— identify 必须如实说不知道。
    #[test]
    fn unidentifiable_package_has_no_slot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let archive = build_zip(
            dir.path(),
            "mystery.zip",
            &[("maps/abc.SC2Map", b"stub".as_slice())],
        );

        let inspection = inspect_package(&archive).expect("预检应成功");
        assert!(!inspection.campaign_type.is_actionable());
        assert!(inspection.suggested_slot.is_none());
        assert!(inspection.identification.is_none());
    }

    /// 槽位合法性：认不出来的 slug 必须被拒（否则会算出一条假路径）。
    #[test]
    fn unknown_slots_are_rejected() {
        assert!(CampaignType::from_slug("reborn").is_none());
        assert_eq!(
            CampaignType::from_slug("custom"),
            Some(CampaignType::Custom)
        );
    }

    /// 落点：官方战役按子目录展开，自制战役进 CustomCampaigns，模组一律进 Mods。
    #[test]
    fn expands_payloads_per_placement() {
        let map = payload(PayloadTarget::Map {
            name: "paiur01.SC2Map".to_string(),
        });
        let module = Payload {
            is_mod: true,
            ..payload(PayloadTarget::Mod {
                name: "X.SC2Mod".to_string(),
            })
        };

        let official = Placement::of("lotv", "reborn", Some("void"));
        let items = expand_payloads(&[map.clone(), module.clone()], &official);
        assert_eq!(
            items[0].target.as_deref(),
            Some("Maps/Campaign/void/paiur01.SC2Map")
        );
        assert_eq!(items[1].target.as_deref(), Some("Mods/X.SC2Mod"));

        let custom = Placement::of("custom", "qunyou", None);
        let items = expand_payloads(&[map, module], &custom);
        assert_eq!(
            items[0].target.as_deref(),
            Some("Maps/CustomCampaigns/qunyou/paiur01.SC2Map")
        );
        assert_eq!(items[1].target.as_deref(), Some("Mods/X.SC2Mod"));
        assert_eq!(placement_folder(&custom).as_deref(), Some("qunyou"));
        assert_eq!(official.base(), "Maps/Campaign");
    }

    /// 参数解析：命令、路径、--pretty 各就各位。
    #[test]
    fn splits_command_paths_and_flags() {
        let args: Vec<String> = ["--pretty", "inspect", "a.zip", "b.zip"]
            .iter()
            .map(|item| item.to_string())
            .collect();

        let mut pretty = false;
        let mut command: Option<String> = None;
        let mut paths: Vec<PathBuf> = Vec::new();
        for arg in &args {
            match arg.as_str() {
                "--pretty" => pretty = true,
                other if command.is_none() => command = Some(other.to_string()),
                other => paths.push(PathBuf::from(other)),
            }
        }

        assert!(pretty);
        assert_eq!(command.as_deref(), Some("inspect"));
        assert_eq!(paths, vec![PathBuf::from("a.zip"), PathBuf::from("b.zip")]);
    }
}
