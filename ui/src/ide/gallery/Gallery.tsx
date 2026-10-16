import { useState, type ReactNode } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import Checkbox from "@jetbrains/ring-ui-built/components/checkbox/checkbox";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import Link from "@jetbrains/ring-ui-built/components/link/link";
import Loader from "@jetbrains/ring-ui-built/components/loader/loader";
import ProgressBar from "@jetbrains/ring-ui-built/components/progress-bar/progress-bar";
import Select from "@jetbrains/ring-ui-built/components/select/select";
import { Tab, Tabs } from "@jetbrains/ring-ui-built/components/tabs/tabs";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";
import Toggle from "@jetbrains/ring-ui-built/components/toggle/toggle";
import Tooltip from "@jetbrains/ring-ui-built/components/tooltip/tooltip";
import addIcon from "@jetbrains/icons/add";
import closeIcon from "@jetbrains/icons/close";
import folderIcon from "@jetbrains/icons/folder";
import searchIcon from "@jetbrains/icons/search";

const CHOICES = [
  { key: "wol", label: "自由之翼改版" },
  { key: "hots", label: "虫群之心改版" },
  { key: "lotv", label: "虚空之遗改版" },
];

/** 展台里那排文档标签的初始内容（够多才看得出溢出折叠）。 */
const DEMO_DOCS = ["Base.SC2Data", "说明.txt", "UnitData.xml", "README.md", "ComponentList.SC2Components"];

function Card({ title, note, children }: { title: string; note: string; children: ReactNode }) {
  return (
    <section className="gcard">
      <h2 className="gcard__title">{title}</h2>
      <p className="gcard__note">{note}</p>
      {children}
    </section>
  );
}

/** 一行：左边固定宽度的名字，右边摆这个控件的各个变体。 */
function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="grow">
      <span className="glabel">{label}</span>
      {children}
    </div>
  );
}

/**
 * 控件展台：`ide.html#gallery` 时**只**渲染这一页。
 *
 * 用户侧的「开发者房间」（AGENTS §18.1）用 iframe 套它，所以这里：
 * 不带工具栏、不碰任何 Tauri 能力 —— 浏览器演示模式下也照样能看。
 * 每个控件都配一行小字说明本项目在哪用它。
 */
export function Gallery() {
  const [text, setText] = useState("示例文本");
  const [choice, setChoice] = useState("hots");
  const [on, setOn] = useState(true);
  const [checked, setChecked] = useState(true);
  const [docs, setDocs] = useState(DEMO_DOCS);
  const [doc, setDoc] = useState(DEMO_DOCS[0]);

  return (
    <div className="gallery">
      <header className="gallery__head">
        <h1 className="gallery__title">Ring UI 控件展台</h1>
        <p className="gallery__note">
          开发者页（<code>ide.html</code>）用到的每个 Ring UI 控件都在这里摆一份，含变体与禁用态。
          只在地址带 <code>#gallery</code> 时出现，是给用户侧「开发者房间」用 iframe 嵌的那一页。
        </p>
      </header>

      <Card
        title="Button"
        note="工具栏（返回启动器 / 重新扫描 / 添加目录 / 导出压缩包）、包信息里的「选图片…」「选 PDF…」、右键菜单项、离开确认框。"
      >
        <Row label="变体">
          <Button>默认</Button>
          <Button primary>primary</Button>
          <Button danger>danger</Button>
          <Button primary danger>primary + danger</Button>
        </Row>
        <Row label="状态">
          <Button icon={addIcon}>带图标</Button>
          <Button disabled>disabled</Button>
          <Button loader>loader</Button>
        </Row>
        <Row label="IconButton">
          <Button icon={searchIcon} title="搜索" />
          <Button icon={closeIcon} title="关闭" />
          <Button icon={folderIcon} active title="游戏目录（选中态）" />
        </Row>
      </Card>

      <Card
        title="Input"
        note="工具栏的包名与搜索框、包信息表单的每个字段、说明用的多行输入、底部日志里的提交说明。"
      >
        <Row label="常态">
          <Input size={Size.M} value={text} onChange={(event) => setText(event.target.value)} />
        </Row>
        <Row label="带图标">
          <Input
            size={Size.M}
            icon={searchIcon}
            value={text}
            onChange={(event) => setText(event.target.value)}
            placeholder="搜索"
          />
        </Row>
        <Row label="disabled">
          <Input size={Size.M} value="禁用态" disabled onChange={() => {}} />
        </Row>
        <Row label="multiline">
          <Input multiline rows={3} autogrow={false} value={text} onChange={(event) => setText(event.target.value)} />
        </Row>
      </Card>

      <Card title="Select" note="包信息里的「归属战役 / 类型 / 主地图」三个下拉。">
        <Row label="常态">
          <Select
            multiple={false}
            size={Size.M}
            data={CHOICES}
            selected={CHOICES.find((item) => item.key === choice) ?? null}
            onChange={(item) => setChoice(item ? String(item.key) : "")}
          />
        </Row>
        <Row label="disabled">
          <Select
            multiple={false}
            size={Size.M}
            disabled
            data={CHOICES}
            selected={CHOICES[0]}
            onChange={() => {}}
          />
        </Row>
      </Card>

      <Card
        title="Toggle"
        note="本页暂时没有布尔开关可换 —— 先摆在这里备查：以后新加开关一律用它，不再手搓。"
      >
        <Row label="常态">
          <Toggle checked={on} onChange={(event) => setOn(event.target.checked)}>
            启用
          </Toggle>
        </Row>
        <Row label="disabled">
          <Toggle checked disabled onChange={() => {}}>
            禁用
          </Toggle>
        </Row>
      </Card>

      <Card
        title="Checkbox"
        note="文件树每一行的勾选：目录用 indeterminate 表示「部分选中」，这正是换掉手搓那个三态复选框的原因。"
      >
        <Row label="常态">
          <Checkbox checked={checked} onChange={(event) => setChecked(event.target.checked)} label="普通" />
        </Row>
        <Row label="部分选中">
          <Checkbox indeterminate onChange={() => {}} label="目录半选" />
        </Row>
        <Row label="disabled">
          <Checkbox disabled onChange={() => {}} label="禁用" />
        </Row>
      </Card>

      <Card
        title="Tooltip"
        note="文件树里勾选框与「空 / 外部」标签的说明、工具栏包名输入框的提示 —— 都从这里来。"
      >
        <Row label="常态">
          <Tooltip title="这是 Ring UI 的 Tooltip">
            <Button>悬停看我</Button>
          </Tooltip>
        </Row>
      </Card>

      <Card
        title="Loader / ProgressBar"
        note="Loader 用在扫描目录、打开文件时的等待；ProgressBar 用在状态栏的导出进度（不再手搓进度条）。"
      >
        <Row label="Loader">
          <Loader message="正在扫描游戏目录…" />
        </Row>
        <Row label="ProgressBar">
          <ProgressBar className="gbar" value={3} max={8} label="导出进度" />
        </Row>
      </Card>

      <Card
        title="Link / Tag"
        note="Link 本页暂无去处（返回启动器是 Button），先摆着备查；Tag 用在文件树的「空 / 外部」与提交记录的「最新」。"
      >
        <Row label="Link">
          <Link href="#gallery" onClick={(event) => event.preventDefault()}>
            Ring UI Link
          </Link>
        </Row>
        <Row label="Tag">
          <Tag>空</Tag>
          <Tag tagType={Tag.Type.WARNING}>外部</Tag>
          <Tag tagType={Tag.Type.MAIN}>最新</Tag>
        </Row>
      </Card>

      <Card
        title="Tabs"
        note="文档标签页 —— 开发者页中间那排（workbench/parts/DocTabs.tsx）：每个标签带关闭 ✕，放不下的自动折成「更多」。"
      >
        <Row label="带关闭 + 溢出">
          <div className="gdoctabs">
            <Tabs autoCollapse selected={doc} onSelect={setDoc}>
              {docs.map((name) => (
                <Tab
                  key={name}
                  id={name}
                  title={
                    <span className="doctab">
                      {name}
                      <Button
                        className="doctab__close"
                        icon={closeIcon}
                        title="关闭"
                        onClick={(event) => {
                          // 关闭不是切换：别让点击冒泡到标签本身
                          event.stopPropagation();
                          setDocs((prev) => prev.filter((item) => item !== name));
                        }}
                      />
                    </span>
                  }
                />
              ))}
            </Tabs>
          </div>
          <Button onClick={() => setDocs(DEMO_DOCS)}>重置标签</Button>
        </Row>
      </Card>
    </div>
  );
}
