import type { ReactNode } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import ButtonToolbar from "@jetbrains/ring-ui-built/components/button-toolbar/button-toolbar";
import Checkbox from "@jetbrains/ring-ui-built/components/checkbox/checkbox";
import ControlLabel, { LabelType } from "@jetbrains/ring-ui-built/components/control-label/control-label";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import Link from "@jetbrains/ring-ui-built/components/link/link";
import ProgressBar from "@jetbrains/ring-ui-built/components/progress-bar/progress-bar";
import ScrollableSection from "@jetbrains/ring-ui-built/components/scrollable-section/scrollable-section";
import Select from "@jetbrains/ring-ui-built/components/select/select";
import Tag from "@jetbrains/ring-ui-built/components/tag/tag";
import Toggle from "@jetbrains/ring-ui-built/components/toggle/toggle";
import Tooltip from "@jetbrains/ring-ui-built/components/tooltip/tooltip";
import addIcon from "@jetbrains/icons/add";
import closeIcon from "@jetbrains/icons/close";
import folderIcon from "@jetbrains/icons/folder";
import searchIcon from "@jetbrains/icons/search";

import type { DemoState } from "./state";
import { Card, DialogDemo, MenuDemo, Row, TabsDemo, TagsDemo } from "./demos";

const CHOICES = [
  { key: "wol", label: "自由之翼改版" },
  { key: "hots", label: "虫群之心改版" },
  { key: "lotv", label: "虚空之遗改版" },
];

/**
 * 每个"点一下会变"的控件都登记在这里 —— 展台自己会核对（见 state.ts 的自检），
 * 免得新控件进来只摆了个不会动的样子货。键与 state.ts 的 DEMO_KEYS 一一对应。
 */
export const DEMO_CONTROLS = [
  "input-normal",
  "input-icon",
  "input-multiline",
  "select-normal",
  "toggle-normal",
  "checkbox-normal",
  "tabs-docs",
  "tags-input",
  "dialog",
  "button-variants",
  "button-states",
  "button-icon",
  "button-loader-text",
  "button-toolbar",
  "input-disabled",
  "select-disabled",
  "toggle-disabled",
  "checkbox-partial",
  "checkbox-disabled",
  "tooltip",
  "progress-bar",
  "link",
  "tag",
  "popup-menu",
  "scrollable-section",
];

/** 展台的全部卡片。控件摆在这儿，业务页面的用法写在每张卡的说明里。 */
export function GallerySections({ state }: { state: DemoState }): ReactNode {
  return (
    <>
      <Card
        title="Button"
        note="工具栏（返回启动器 / 重新扫描 / 添加目录 / 导出压缩包）、包信息里的「选图片…」「选 PDF…」、离开确认框、状态栏的重试。"
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
        </Row>
        <Row label="IconButton">
          <Button icon={searchIcon} title="搜索" />
          <Button icon={closeIcon} title="关闭" />
          <Button icon={folderIcon} active title="游戏目录（选中态）" />
        </Row>
        <Row label="处理中 loader">
          {/* 忙碌态：按钮自己转圈，别只靠 disabled（工具栏的导出 / 重新扫描、文件树的
              刷新、日志的提交都是这样用的） */}
          <Button primary loader>
            导出中…
          </Button>
          <Button loader>刷新中…</Button>
          <Button loader disabled>
            提交中…
          </Button>
        </Row>
      </Card>

      <Card
        title="ButtonToolbar"
        note="顶部工具栏就靠它分组：左边「返回 / 重新扫描 / 添加目录」，右边搜索 + 导出。组内间距由它给，组间用页面的 gap。"
      >
        <Row label="一组（关键状态）">
          <ButtonToolbar>
            <Button icon={folderIcon}>返回启动器</Button>
            <Button icon={addIcon} loader>
              重新扫描
            </Button>
            <Button icon={addIcon}>添加目录…</Button>
          </ButtonToolbar>
        </Row>
      </Card>

      <Card
        title="Input"
        note="工具栏的包名与搜索框、包信息表单的每个字段、说明用的多行输入、底部日志里的提交说明。"
      >
        <Row label="常态">
          <Input
            key="input-normal"
            size={Size.M}
            value={state.text}
            onChange={(event) => state.setText(event.target.value)}
          />
        </Row>
        <Row label="带图标">
          <Input
            key="input-icon"
            size={Size.M}
            icon={searchIcon}
            value={state.text}
            onChange={(event) => state.setText(event.target.value)}
            placeholder="搜索"
          />
        </Row>
        <Row label="disabled">
          <Input size={Size.M} value="禁用态" disabled onChange={() => {}} />
        </Row>
        <Row label="multiline">
          <Input
            key="input-multiline"
            multiline
            rows={3}
            autogrow={false}
            value={state.text}
            onChange={(event) => state.setText(event.target.value)}
          />
        </Row>
      </Card>

      <Card title="Select" note="包信息里的「归属战役 / 类型 / 主地图」三个下拉。">
        <Row label="常态">
          <Select
            multiple={false}
            size={Size.M}
            data={CHOICES}
            selected={CHOICES.find((item) => item.key === state.choice) ?? null}
            onChange={(item) => state.setChoice(item ? String(item.key) : "")}
          />
        </Row>
        <Row label="disabled">
          <Select multiple={false} size={Size.M} disabled data={CHOICES} selected={CHOICES[0]} onChange={() => {}} />
        </Row>
      </Card>

      <Card
        title="TagsInput"
        note="包信息的「标签」与「依赖模组」—— 从前是逗号分隔的单行输入，现在一个值一枚可删的标签（回车或空格加一个）。"
      >
        <Row label="常态">
          <TagsDemo state={state} />
        </Row>
      </Card>

      <Card
        title="ControlLabel"
        note="包信息表单每一行的标签（MetaPane 的 Row）。竖排表单用 type=form，这里是横排 + 固定 62px 宽，所以 ide.css 把它的下边距归零。"
      >
        <Row label="排成一行">
          <ControlLabel className="field__label" type={LabelType.FORM}>
            名称
          </ControlLabel>
          <Input className="field__ctl" size={Size.S} value="示例战役" onChange={() => {}} />
        </Row>
      </Card>

      <Card
        title="Toggle"
        note="本页暂时没有布尔开关可换 —— 先摆在这里备查：以后新加开关一律用它，不再手搓。"
      >
        <Row label="常态">
          <Toggle checked={state.on} onChange={(event) => state.setOn(event.target.checked)}>
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
          <Checkbox checked={state.checked} onChange={(event) => state.setChecked(event.target.checked)} label="普通" />
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
        title="ProgressBar"
        note="状态栏的导出进度（EditorPane 底部）。"
      >
        <Row label="常态">
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
        title="PopupMenu / DropdownMenu"
        note="右键菜单（workbench/parts/MenuLayer.tsx）：文件行、提交记录、空白处三处共用一份。定位、贴边翻转、点外关闭、Esc 与上下键都归它。"
      >
        <Row label="菜单项">
          <MenuDemo />
        </Row>
      </Card>

      <Card
        title="Tabs"
        note="文档标签页 —— 开发者页中间那排（workbench/parts/DocTabs.tsx）：每个标签带关闭 ✕，放不下的自动折成「更多」。"
      >
        <Row label="带关闭 + 溢出">
          <TabsDemo state={state} />
        </Row>
      </Card>

      <Card
        title="Dialog"
        note="打包没完就想走的那个确认框（workbench/parts/LeaveGuard.tsx）。遮罩、焦点陷阱、Esc 都是它自带的 —— 换掉手搓遮罩就是为了键盘能进去。"
      >
        <Row label="模态">
          <DialogDemo state={state} />
        </Row>
      </Card>

      <Card
        title="ScrollableSection"
        note="内容自己滚的那些面板（编辑器中间的提示、日志的列表与详情）用它包一层：滚出边界的那一侧会自动出现一条阴影线。"
      >
        <Row label="溢出时出阴影">
          <ScrollableSection className="gscroll">
            {Array.from({ length: 20 }, (_, index) => (
              <div key={index}>第 {index + 1} 行 —— 滚动看左边的阴影线</div>
            ))}
          </ScrollableSection>
        </Row>
      </Card>
    </>
  );
}
