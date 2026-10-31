//! 主地图（自制战役的游玩入口）解析：**不确定就不猜**。

use super::known;
use super::model::{MainMapChoice, MapEntry};

/// 从地图列表里挑出该用哪张作为入口。
///
/// 规则（**不确定就不猜**）：
///
/// | 情况 | 结果 |
/// | --- | --- |
/// | 设了主地图，且**确实存在** | 用它 |
/// | 设了，但找不到 | 给个警告，退回「没定」 |
/// | 是**知名复刻战役**（见 `known`） | 用它表里的入口地图 |
/// | 没设，且整个版本只有一张地图 | 自动用它（省用户一次点击） |
/// | 没设，但包根有一张 **入口地图** | 自动用它（见下） |
/// | 其余 | 没定，让用户自己挑 |
///
/// **打表排在启发式前面** —— 知名复刻的入口是固定的，表比猜可靠。
/// 实测教训：SCMR 那种包里躺着第三方战役的启动器
/// （`Enslavers Redux Campaign Launcher`，在 `8. Enslavers Redux/` 里），
/// 用户在列表里挑很容易挑错那个；表直接点名自己的入口。
///
/// 「入口地图」的判定很窄：**放在包根**（不在章节子目录里）**且名字里带
/// launcher / 启动 / 入口**。真实样本教我们的 —— 复刻战役（SCMR）会在包根放一张
/// `SCMR Campaign Launcher.SC2Map`，那就是它的入口，不认出来用户就得在
/// 138 张地图里自己翻。
///
/// 这里敢猜是因为**猜错很便宜**：界面上一次点击就能改，不像「这个包属于哪部战役」
/// 那种猜错了整个装错地方。
/// 路径比较**忽略大小写与斜杠方向** —— 作者在包里写 `1. Rebel Yell\a.SC2Map`
/// 还是 `1. Rebel Yell/a.SC2Map` 都得认。
pub fn resolve_main_map(maps: &[MapEntry], declared: Option<&str>) -> MainMapChoice {
    let normalize = |value: &str| value.replace('\\', "/").to_lowercase();

    if let Some(wanted) = declared.map(str::trim).filter(|value| !value.is_empty()) {
        let wanted_key = normalize(wanted);
        // 先按完整相对路径比；再退一步只按文件名比（作者可能只写了文件名）
        let hit = maps
            .iter()
            .find(|map| normalize(&map.path) == wanted_key)
            .or_else(|| {
                maps.iter().find(|map| {
                    normalize(&map.name) == wanted_key
                        || normalize(map.path.rsplit('/').next().unwrap_or("")) == wanted_key
                })
            });

        return match hit {
            Some(map) => MainMapChoice {
                path: Some(map.path.clone()),
                automatic: false,
                warning: None,
            },
            None => MainMapChoice {
                path: None,
                automatic: false,
                warning: Some(format!(
                    "找不到「{wanted}」这张地图，请自己挑一张作为启动入口"
                )),
            },
        };
    }

    // **知名复刻战役：打表认它的入口。**
    //
    // 排在启发式前面 —— 这些战役的入口是固定的，表比猜可靠。
    // 而且表能处理启发式处理不了的：一个知名复刻包里往往还躺着**别的**
    // 战役（SCMR 包里就有 Enslavers Redux 的启动器），用户很容易挑错。
    if let Some((path, remake)) = known::find_launcher(maps) {
        let _ = remake;
        return MainMapChoice {
            path: Some(path),
            automatic: true,
            warning: None,
        };
    }

    // 没声明：只有一张的话直接用它，省用户一次点击
    if maps.len() == 1 {
        return MainMapChoice {
            path: Some(maps[0].path.clone()),
            automatic: true,
            warning: None,
        };
    }

    // 没声明，但包根摆了一张「入口地图」—— 多半就是它
    if let Some(map) = maps.iter().find(|map| looks_like_entry_map(map)) {
        return MainMapChoice {
            path: Some(map.path.clone()),
            automatic: true,
            warning: None,
        };
    }

    MainMapChoice::unset()
}

/// 这张地图看起来是不是整部战役的**入口**。
///
/// 两条都满足才算：**在包根**（路径里没有子目录）**且名字里有入口字样**。
/// 条件收得这么窄，是因为放宽一点点就会把普通关卡认成入口。
fn looks_like_entry_map(map: &MapEntry) -> bool {
    let name = map.name.to_lowercase();
    let at_root = !map.path.contains('/') && !map.path.contains('\\');
    at_root
        && (name.contains("launcher")
            || name.contains("启动")
            || name.contains("入口")
            || name.contains("entry"))
}
