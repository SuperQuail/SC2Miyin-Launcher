import { useState, type ReactNode } from "react";

import type { SlotView, Variant } from "../../../api/types";
import { SlotCard } from "../../components/SlotCard";
import { VariantCard } from "../../components/VariantCard";
import { Button, Choice, Field, Progress, Sheet, Toast, type ToastKind } from "../../ui";

/**
 * 开发者房间 · **用户侧**。
 *
 * 这一页展示用户侧（index.html）的控件：先是控件层（Button / Field / Sheet），
 * 再是共用组件（战役卡 / 版本卡），最后是「还没控件化的」清单。
 *
 * 规矩见 AGENTS §18.1：**没有在房间里出现过的控件，不许上生产界面。**
 */

/** 一份假的版本数据，只为把卡片渲染出来（房间里不碰真实库）。 */
function fakeVariant(over: Partial<Variant> = {}): Variant {
  return {
    id: "demo-variant",
    name: "演示版本",
    author: "示例作者",
    version: "1.4",
    description: null,
    format: "miyin" as Variant["format"],
    imported_at: 0,
    source: null,
    map_count: 14,
    mod_count: 2,
    size_bytes: 724_800_000,
    target_sub: null,
    cover: null,
    tags: ["重制", "中文"],
    registration_id: null,
    declared_mods: [],
    doc: null,
    ...over,
  };
}

function fakeSlot(over: Partial<SlotView> = {}): SlotView {
  return {
    slug: "hots",
    display_name: "虫群之心",
    sub_directory: "swarm",
    variants: [fakeVariant()],
    active: null,
    active_name: null,
    notice: null,
    ...over,
  };
}

function Card({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className="room__card">
      <h3 className="room__card-title">{title}</h3>
      {note && <p className="room__card-note">{note}</p>}
      {children}
    </section>
  );
}

export function UserSidePage() {
  const [sheet, setSheet] = useState(false);
  const [backdropSheet, setBackdropSheet] = useState(false);
  const [demoText, setDemoText] = useState("演示文本");
  const [dismissed, setDismissed] = useState("");
  // Toast 演示：换 key 强制重播入场动画（同一条提示再点一次也要重播）
  const [toastKind, setToastKind] = useState<ToastKind | null>("success");
  // Choice 演示：安装页那个「覆盖更新 / 重命名后导入」就是它
  const [conflict, setConflict] = useState<"overwrite" | "rename">("overwrite");
  const [toastTick, setToastTick] = useState(0);

  return (
    <>
      <Card title="控件层 · Button" note="react/ui/Button —— 变体、尺寸、禁用态。">
        <div className="room__row">
          <span className="room__label">variant</span>
          <Button variant="primary">primary</Button>
          <Button variant="tonal">tonal</Button>
          <Button variant="outline">outline</Button>
          <Button>text</Button>
          <Button variant="danger">danger</Button>
        </div>
        <div className="room__row">
          <span className="room__label">size / 禁用</span>
          <Button variant="primary" size="lg">
            lg
          </Button>
          <Button variant="primary" disabled>
            禁用
          </Button>
        </div>
      </Card>

      <Card title="控件层 · Field" note="react/ui/Field —— 标签、提示、错误、整行。">
        <div className="room__fields">
          <Field label="普通输入">
            <input type="text" value={demoText} onChange={(event) => setDemoText(event.target.value)} />
          </Field>
          <Field label="带提示" hint="这里写一行说明。">
            <input type="text" defaultValue="" />
          </Field>
          <Field label="出错" error="这个名字不合法。">
            <input type="text" defaultValue="???" />
          </Field>
          <Field label="下拉">
            <select defaultValue="b">
              <option value="a">选项 A</option>
              <option value="b">选项 B</option>
            </select>
          </Field>
        </div>
        <Field label="整行（wide）" wide hint="说明、备注这种长字段用它。">
          <textarea rows={2} defaultValue="" />
        </Field>
      </Card>

      <Card
        title="控件层 · Choice / Progress / 处理中按钮 / 属性行"
        note="react/ui —— 单选得看得出是单选；进度条以前只活在 settings.css 里，别处要用就再手搓一条。"
      >
        <div className="room__col">
          <Choice
            label="库里已有这一版，怎么办"
            value={conflict}
            onChange={setConflict}
            options={[
              { value: "overwrite", label: "覆盖更新", hint: "沿用原目录名" },
              { value: "rename", label: "重命名后导入", hint: "两个版本并存" },
            ]}
            hint="覆盖更新会替换掉旧版本的文件；重命名则两份都留着，各自启用互不影响。"
          />
          <Progress value={62} label="正在打包" detail="123 / 2331" />
          <Progress value={null} label="不确定进度（没有事件流时用它）" />
        </div>
        <div className="room__row">
          <Button variant="primary" loading>
            安装中…
          </Button>
          <Button variant="outline" loading>
            重新扫描
          </Button>
          <span className="room__note">loading = 转圈 + 禁用 + aria-busy</span>
        </div>
        <Field label="整合包名称" row>
          <input defaultValue="Starcraft Mass Recall" />
        </Field>
        <Field label="版本" row hint="属性表那样：标签左、值右">
          <input defaultValue="8.0" />
        </Field>
      </Card>

      <Card
        title="控件层 · Toast"
        note="react/ui/Toast —— 四种语气、右下角滑入、淡出退场、自动消失倒计时条。点一次看一次动画。"
      >
        <div className="room__row">
          {(["success", "info", "warning", "error"] as ToastKind[]).map((kind) => (
            <Button
              key={kind}
              variant={kind === "error" ? "danger" : "outline"}
              onClick={() => {
                setToastKind(kind);
                setToastTick((tick) => tick + 1);
              }}
            >
              {kind}
            </Button>
          ))}
        </div>
        <p className="room__card-note">
          这就是右下角那条提示本身（举高到 9 秒，方便你看清倒计时条；生产里是 4.2 秒）。
        </p>
        {toastKind && (
          <Toast
            key={toastTick}
            kind={toastKind}
            message={"演示：" + toastKind + " 语气的提示，比如「已启用 Starcraft Mass Recall 8.0」"}
            lifeMs={9000}
            onClose={() => setToastKind(null)}
          />
        )}
      </Card>

      <Card
        title="控件层 · Sheet"
        note="react/ui/Sheet —— 遮罩弹窗。点遮罩关不关由调用方决定（这里是两种都演一遍）。"
      >
        <div className="room__row">
          <Button variant="primary" onClick={() => setSheet(true)}>
            打开（点遮罩可关）
          </Button>
          <Button onClick={() => setBackdropSheet(true)}>打开（点遮罩不关）</Button>
        </div>
        {dismissed && <p className="room__card-note">上一次的关闭来源：{dismissed}</p>}
      </Card>

      <Card
        title="通用组件 · SlotCard / VariantCard"
        note="react/components —— 用一份假数据渲染（房间里不碰真实库）。"
      >
        <div className="room__stage">
          <SlotCard slot={fakeSlot()} onOpen={() => setDismissed("")} />
          <SlotCard
            slot={fakeSlot({
              active: "demo-variant",
              active_name: "演示版本",
              variants: [fakeVariant(), fakeVariant({ id: "v2" })],
            })}
            onOpen={() => setDismissed("")}
          />
          <VariantCard
            slot={fakeSlot()}
            variant={null}
            active={false}
            selected={false}
            onPick={() => setDismissed("")}
            onActivate={() => setDismissed("")}
            onDrop={() => setDismissed("")}
            onMenu={() => setDismissed("")}
          />
          <VariantCard
            slot={fakeSlot()}
            variant={fakeVariant()}
            active
            selected
            onPick={() => setDismissed("")}
            onActivate={() => setDismissed("")}
            onDrop={() => setDismissed("")}
            onMenu={() => setDismissed("")}
          />
        </div>
      </Card>

      <Card
        title="待迁移（还没控件化的）"
        note="这一栏只出不进：新写的界面要么用已有控件，要么先在房间里做出新控件。"
      >
        <p className="room__card-note">
          没有了 —— 用户侧的外壳（<code>App.vue</code>）、全局状态（<code>useLauncher</code>）
          与右键菜单宿主（<code>ContextMenu.vue</code>）都已经换成 React，Vue 依赖已删。
        </p>
      </Card>

      {sheet && (
        <Sheet
          title="演示：点遮罩可关"
          onClose={() => {
            setDismissed("点遮罩 / 取消");
            setSheet(false);
          }}
          actions={
            <>
              <Button
                onClick={() => {
                  setDismissed("取消");
                  setSheet(false);
                }}
              >
                取消
              </Button>
              <Button
                variant="primary"
                onClick={() => {
                  setDismissed("确定");
                  setSheet(false);
                }}
              >
                确定
              </Button>
            </>
          }
        >
          <p className="sheet__text">普通弹窗：点遮罩、点取消都能关。</p>
        </Sheet>
      )}

      {backdropSheet && (
        <Sheet title="演示：点遮罩不关" dismissOnBackdrop={false} onClose={() => setDismissed("（点遮罩无效）")}>
          <p className="sheet__text">
            编辑表单这类弹窗用它 —— 用户点到外面不该把填了一半的东西丢掉。
          </p>
          <div className="room__row">
            <Button
              variant="primary"
              onClick={() => {
                setDismissed("自己关掉的");
                setBackdropSheet(false);
              }}
            >
              关掉
            </Button>
          </div>
        </Sheet>
      )}
    </>
  );
}
