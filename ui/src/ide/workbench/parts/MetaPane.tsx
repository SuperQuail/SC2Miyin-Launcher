import type { ReactNode } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import ControlLabel, { LabelType } from "@jetbrains/ring-ui-built/components/control-label/control-label";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import Select from "@jetbrains/ring-ui-built/components/select/select";
import TagsInput from "@jetbrains/ring-ui-built/components/tags-input/tags-input";

import { formatBytes, joinList, parseList } from "../../lib/format";
import type { DevMeta } from "../../lib/settings";

const CAMPAIGN_OPTIONS = [
  { key: "", label: "自制战役（不依附官方）" },
  { key: "wol", label: "自由之翼改版" },
  { key: "hots", label: "虫群之心改版" },
  { key: "lotv", label: "虚空之遗改版" },
  { key: "nova", label: "诺娃隐秘行动改版" },
];

const KIND_OPTIONS = [
  { key: "campaign", label: "campaign（战役本体）" },
  { key: "patch", label: "patch（覆盖层）" },
];

/** 一个下拉：Ring UI 的 Select 吃 `{ key, label }` 数组，值就是 key。 */
function Choice({
  value,
  options,
  onChange,
}: {
  value: string;
  options: { key: string; label: string }[];
  onChange: (value: string) => void;
}) {
  const selected = options.find((option) => option.key === value) ?? null;
  return (
    <Select
      multiple={false}
      size={Size.S}
      className="field__ctl"
      data={options}
      selected={selected}
      onChange={(item) => onChange(item ? String(item.key) : "")}
    />
  );
}

/**
 * 一行表单：左边是 Ring UI 的 `ControlLabel`（点标签聚焦控件），右边是控件。
 * Select / TagsInput 这类不带 htmlFor 的控件用 `labelId` 走 aria-labelledby。
 */
function Row({
  label,
  full = false,
  labelId,
  children,
}: {
  label: string;
  full?: boolean;
  labelId?: string;
  children: ReactNode;
}) {
  const labelNode = (
    <ControlLabel className="field__label" type={LabelType.FORM} id={labelId}>
      {label}
    </ControlLabel>
  );
  if (labelId) {
    return (
      <div className={"field" + (full ? " field--full" : "")}>
        {labelNode}
        {children}
      </div>
    );
  }
  return (
    <label className={"field" + (full ? " field--full" : "")}>
      {labelNode}
      {children}
    </label>
  );
}

/**
 * 逗号分隔的那些字段（标签 / 依赖模组）用 Ring UI 的 `TagsInput`：
 * 每个值是一枚可删的标签，回车或空格加一个，不用再让人猜"逗号是中文还是英文"。
 *
 * 控件内部的建议列表我们不用（`dataSource` 给空），但 `allowAddNewTags` 得开 ——
 * 不然打进去的字加不成标签。
 */
function TagSet({ value, onChange, placeholder }: { value: string; onChange: (next: string) => void; placeholder: string }) {
  const tags = parseList(value).map((tag) => ({ key: tag, label: tag }));
  const labels = tags.map((tag) => tag.label);
  return (
    <TagsInput
      className="field__ctl"
      size={Size.S}
      tags={tags}
      placeholder={placeholder}
      allowAddNewTags
      dataSource={() => []}
      // 库里两个回调都把标签包在 `{ tag }` 里递出来（ToggleTagParams）—— 运行时的
      // `tag` 就是我们传进去的 `{ key, label }`，类型声明里少写了 label
      onAddTag={(params) => onChange(joinList([...labels, String(params.tag.label ?? "")]))}
      onRemoveTag={(params) => onChange(joinList(labels.filter((label) => label !== params.tag.label)))}
    />
  );
}

export type MetaPaneProps = {
  pkg: string;
  onPkg: (value: string) => void;
  meta: DevMeta;
  onMeta: (meta: DevMeta) => void;
  /** 版本号校验的错误文案；空串表示没问题 */
  versionError: string;
  coverPath: string;
  docPath: string;
  onPickCover: () => void;
  onPickDoc: () => void;
  /** 正在挑封面 / 说明书 —— 按钮转 loader */
  pickingCover: boolean;
  pickingDoc: boolean;
  /** 已勾选的 .SC2Map 路径（选主地图用） */
  maps: string[];
  pickedCount: number;
  pickedBytes: number;
};

/** 右栏：包信息表单（导出时写进 metadata.json）。 */
export function MetaPane({
  pkg,
  onPkg,
  meta,
  onMeta,
  versionError,
  coverPath,
  docPath,
  onPickCover,
  onPickDoc,
  pickingCover,
  pickingDoc,
  maps,
  pickedCount,
  pickedBytes,
}: MetaPaneProps) {
  const mainMapOptions = [
    { key: "", label: maps.length ? "不指定" : "先勾选一张 .SC2Map" },
    ...maps.map((path) => ({ key: path, label: path })),
  ];

  return (
    <section className="pane pane--meta">
      <header className="pbar">
        <span className="ptitle">包信息</span>
        <span className="psub">导出时写进 metadata.json</span>
      </header>
      <div className="pbody">
        <Row label="名称">
          <Input
            className="field__ctl"
            size={Size.S}
            value={pkg}
            onChange={(event) => onPkg(event.target.value)}
            placeholder="必填"
          />
        </Row>
        <Row label="作者">
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.author}
            onChange={(event) => onMeta({ ...meta, author: event.target.value })}
            placeholder="你的名字"
          />
        </Row>
        <Row label="版本">
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.version}
            error={versionError || null}
            onChange={(event) => onMeta({ ...meta, version: event.target.value })}
            placeholder="比如 1.0 / v8.0.1"
          />
        </Row>
        <Row label="注册 ID">
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.id}
            onChange={(event) => onMeta({ ...meta, id: event.target.value })}
            placeholder="同一个战役的多个版本靠它认亲"
          />
        </Row>
        <Row label="标签">
          <TagSet
            value={meta.tags}
            onChange={(tags) => onMeta({ ...meta, tags })}
            placeholder="回车加一个，比如 重制"
          />
        </Row>
        <Row label="归属战役" labelId="meta-campaign">
          <Choice
            value={meta.campaign}
            options={CAMPAIGN_OPTIONS}
            onChange={(value) => onMeta({ ...meta, campaign: value })}
          />
        </Row>
        <Row label="类型" labelId="meta-kind">
          <Choice value={meta.kind} options={KIND_OPTIONS} onChange={(value) => onMeta({ ...meta, kind: value })} />
        </Row>
        <Row label="模组 ID">
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.modid}
            onChange={(event) => onMeta({ ...meta, modid: event.target.value })}
            placeholder="模组包才填（modid）"
          />
        </Row>
        <Row label="依赖模组">
          <TagSet
            value={meta.mods}
            onChange={(mods) => onMeta({ ...meta, mods })}
            placeholder="回车加一个；这张地图要靠哪些模组"
          />
        </Row>
        <Row label="说明" full>
          <Input
            className="field__ctl"
            multiline
            rows={3}
            autogrow={false}
            value={meta.description}
            onChange={(event) => onMeta({ ...meta, description: event.target.value })}
            placeholder="一两句说清这个包是什么"
          />
        </Row>
        <Row label="封面图" full labelId="meta-cover">
          <Button loader={pickingCover} onClick={onPickCover}>选图片…</Button>
          <span className="field__file">
            {coverPath ? coverPath.split(/[\\/]/).pop() : "没选（用官方美术）"}
          </span>
        </Row>
        <Row label="说明书" full labelId="meta-doc">
          <Button loader={pickingDoc} onClick={onPickDoc}>选 PDF…</Button>
          <span className="field__file">{docPath ? docPath.split(/[\\/]/).pop() : "没选"}</span>
        </Row>
        {meta.campaign === "" && (
          <Row label="主地图" full labelId="meta-mainmap">
            <Choice value={meta.mainMap} options={mainMapOptions} onChange={(value) => onMeta({ ...meta, mainMap: value })} />
          </Row>
        )}
        <p className="hintbox">
          自制战役建议指定主地图 —— 它是玩家从启动器进游戏的入口。
          包内路径保持游戏目录里的相对路径，导入时不用重新猜落点。
        </p>
        <p className="hintbox">
          这个包会包含 <strong>{pickedCount}</strong> 个文件，共 <strong>{formatBytes(pickedBytes) || "0 B"}</strong>。
        </p>
      </div>
    </section>
  );
}
