import { useState } from "react";

import { Button } from "../../../ui";
import { CandASnackbar } from "./cand-a-m3";
import { CandBNoticeCard } from "./cand-b-vscode";
import { CandCSonnerStack } from "./cand-c-sonner";
import { CandDCapsule } from "./cand-d-capsule";
import { CandECard } from "./cand-e-card";
import { AUTO_MS } from "./types";
import "./toast-lab.css";

/**
 * **提示（Toast）样式候选** —— 房间里给用户挑的那一栏（AGENTS §17 的"至少三个版本"）。
 *
 * 每个候选一张小卡：名字、抄谁、关键差别、一个「播一遍」按钮。
 * 想跳过按钮直接看：地址后面加 `?toast=<id>`（a / b / c / d / e），
 * 进房间就自动放一遍 —— 截图脚本走的就是这条路（点按钮截图得先点，太啰嗦）。
 */

const LONG =
  "导入「Starcraft Mass Recall 8.0」时发现库里已经有一个 7.4 —— 两个版本的注册 ID 相同，" +
  "覆盖更新会沿用原来的目录名并替换全部文件，重命名后导入则两份都留在库里各自启用。";

type Spec = {
  id: string;
  name: string;
  copy: string;
  place: string;
  diff: string[];
};

/** 卡片上的说明只用一种强调：\`**这样**\` 的部分加粗，其余原样。 */
function bold(text: string) {
  return text.split("**").map((part, index) =>
    index % 2 === 1 ? <b key={part}>{part}</b> : part,
  );
}

const SPECS: Spec[] = [
  {
    id: "a",
    name: "候选 A · Material 3 Snackbar",
    copy: "Material 3 Snackbar",
    place: "底部居中 · 反色底 · 一条动作按钮",
    diff: [
      "位置从右下角挪到**底部居中**。M3 官方就一个位置、一个语气，最简单。",
      "容器是**反色**的（浅色主题深底浅字、深色主题浅底深字），圆角 **4dp**、阴影 **6dp**。",
      "**不区分语气**：M3 的 snackbar 只有一种容器色，语气最多靠一个前置图标表达。",
      "右侧**一个文字按钮**（InversePrimary），**没有 ✕**；官方明确「同时只显示一条，其余排队」—— 下载/安装这种连续提示别抄这条。",
    ],
  },
  {
    id: "b",
    name: "候选 B · VS Code 通知卡片",
    copy: "VS Code 通知",
    place: "右上角 · 跟随主题的卡片 · 标题 + 正文",
    diff: [
      "位置挪到**右上角**，从右侧滑入（官方 UX 指南写的是「从右下角冒出来」，这里换个角是故意的对照组）。",
      "多了**标题行**：一句话讲清发生了什么，正文再给细节 —— 长文案在这里最好读。",
      "语气只给**警告/失败**一条左窄条，成功不给（官方原话：success 要少用、没事别放动作按钮）。",
      "按钮在正文下面靠右、✕ 常驻；正文超过 3 行截断。",
    ],
  },
  {
    id: "c",
    name: "候选 C · Sonner 式堆叠",
    copy: "Sonner（shadcn 默认提示）",
    place: "左下角 · 卡片贴得紧 · 超过一条往左上叠",
    diff: [
      "位置在**左下角**，和右下角的按钮彻底错开。",
      "同时来两条时**叠成一层**：最新的完整、上一条只露个头并压暗缩小（官方默认可见 3 条）。",
      "官方还有一条没做进来：**鼠标悬停时整叠展开**（现在只做了悬停暂停计时）。",
      "最前面那条有胶囊形动作按钮和 ✕，后面的只露一行标题。",
    ],
  },
  {
    id: "d",
    name: "候选 D · 底部居中胶囊",
    copy: "Ant Design message",
    place: "底部居中 · 一行字 · 无按钮无 ✕",
    diff: [
      "**没有卡片**：深色胶囊浮在底部居中，只有图标 + 一行字（antd 的 message 默认顶部居中，这里挪到底部对比）。",
      "**不给动作按钮、不给 ✕**（antd 的 message 默认也不能手动关），几秒自己走 —— 当「别挡路」的对照组。",
      "antd 另外那把 notification 才是「卡片 + 动作 + ✕」，别把两者记混。",
      "长文案会把它撑成两行，所以长文案还是别用它。",
    ],
  },
  {
    id: "e",
    name: "候选 E · 右下角语气色卡",
    copy: "生产版位置（右下角）",
    place: "右下角 · 图标徽章染色 · 实心动作按钮",
    diff: [
      "**位置照抄生产版**（右下角）—— 只想换长相、不想动位置就选它（Windows 通知实际也在右下角，但微软文档没写这条）。",
      "语气收在**图标徽章**上（浅色底 + 语气色图标），卡片跟随主题，不用反色。",
      "动作按钮是**实心主色、整行**，比现在的文字按钮显眼：撤销/查看这种要人真去点的用它。",
      "留了一个**倒计时条开关**：生产版现在有那条，可以对比着看要不要留（antd / Element Plus 也都有这个东西，但默认是关的）。",
    ],
  },
];

export function ToastCandidates() {
  // `?toast=a|b|c|d|e`：进房间就自动放这一条（截图与自测用）
  const [auto] = useState(() =>
    typeof location === "undefined" ? "" : (new URLSearchParams(location.search).get("toast") ?? ""),
  );
  const [tick, setTick] = useState<Record<string, number>>({});
  const play = (id: string) => setTick((prev) => ({ ...prev, [id]: (prev[id] ?? 0) + 1 }));
  const at = (id: string) => tick[id] ?? 0;

  return (
    <section className="room__card">
      <h3 className="room__card-title">提示样式候选 · 待用户挑一个</h3>
      <p className="room__card-note">
        生产那个 Toast（<code>react/ui/Toast.tsx</code>）没动。这里是另外几种思路，各写各的，
        点卡片或「播一遍」重放一次：入场、停留、退场都会演到（演示时长 6~9 秒。
        地址后面加 <code>?toast=a|b|c|d|e</code> 可以进页面就自动放那一条，
        那种情况下演示时长给到 {AUTO_MS / 1000} 秒，方便慢慢看 / 截图）。
      </p>

      <div className="cand__list">
        {SPECS.map((spec) => (
          <div
            className="cand cand__play"
            key={spec.id}
            role="button"
            tabIndex={0}
            title="点这里播一遍"
            onClick={() => play(spec.id)}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                play(spec.id);
              }
            }}
          >
            <div className="cand__head">
              <h4 className="cand__name">{spec.name}</h4>
              <span className="cand__copy">抄 {spec.copy}</span>
              <span className="cand__place">{spec.place}</span>
            </div>
            <ul className="cand__diff">
              {spec.diff.map((line) => (
                <li key={line}>{bold(line)}</li>
              ))}
            </ul>
            <div className="cand__actions">
              <Button
                variant="outline"
                onClick={(event) => {
                  event.stopPropagation();
                  play(spec.id);
                }}
              >
                播一遍
              </Button>
              {auto === spec.id && <span className="cand__live">← 现在放的就是这条（?toast= 指定）</span>}
            </div>
          </div>
        ))}
      </div>

      {/* 候选本体：fixed 定位，和卡片摆在哪儿无关 */}
      <CandASnackbar
        active={auto === "a"}
        reset={at("a")}
        tone="success"
        message="已启用自我之战 8.0，回到主页就能进游戏了。"
        action={{ label: "查看", onClick: () => {} }}
      />
      <CandBNoticeCard
        active={auto === "b"}
        reset={at("b")}
        tone="error"
        title="导入失败"
        message={LONG}
        action={{ label: "查看日志", onClick: () => {} }}
      />
      <CandCSonnerStack
        active={auto === "c"}
        reset={at("c")}
        items={[
          { tone: "success", title: "已启用自我之战 8.0", message: "回到主页就能进游戏了。" },
          {
            tone: "warning",
            title: "有 1 个版本未启用",
            message: "库里还有 7.4，启用它会先停掉现在这版。",
            action: { label: "撤销", onClick: () => {} },
          },
        ]}
      />
      <CandDCapsule active={auto === "d"} reset={at("d")} tone="info" message="已重新扫描，找到 12 个战役。" />
      <CandECard
        active={auto === "e"}
        reset={at("e")}
        tone="warning"
        title="这条会挡住房间的动画"
        message="举到 8 秒只是让你看清倒计时条；生产里是 4.2 秒。"
        action={{ label: "知道了", onClick: () => {} }}
        timer
      />
    </section>
  );
}