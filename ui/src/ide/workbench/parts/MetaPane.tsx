import type { ReactNode } from "react";

import Button from "@jetbrains/ring-ui-built/components/button/button";
import Input, { Size } from "@jetbrains/ring-ui-built/components/input/input";
import Select from "@jetbrains/ring-ui-built/components/select/select";

import { formatBytes } from "../../lib/format";
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

/** 一行表单：左边固定宽度的标签，右边控件（Input 这类可点标签聚焦的用它）。 */
function Row({ label, full = false, children }: { label: string; full?: boolean; children: ReactNode }) {
  return (
    <label className={"field" + (full ? " field--full" : "")}>
      <span>{label}</span>
      {children}
    </label>
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
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.tags}
            onChange={(event) => onMeta({ ...meta, tags: event.target.value })}
            placeholder="逗号分开，比如 重制, 剧情"
          />
        </Row>
        <div className="field">
          <span>归属战役</span>
          <Choice
            value={meta.campaign}
            options={CAMPAIGN_OPTIONS}
            onChange={(value) => onMeta({ ...meta, campaign: value })}
          />
        </div>
        <div className="field">
          <span>类型</span>
          <Choice value={meta.kind} options={KIND_OPTIONS} onChange={(value) => onMeta({ ...meta, kind: value })} />
        </div>
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
          <Input
            className="field__ctl"
            size={Size.S}
            value={meta.mods}
            onChange={(event) => onMeta({ ...meta, mods: event.target.value })}
            placeholder="逗号分开；这张地图要靠哪些模组"
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
        <div className="field field--full">
          <span>封面图</span>
          <Button onClick={onPickCover}>选图片…</Button>
          <span className="field__file">
            {coverPath ? coverPath.split(/[\\/]/).pop() : "没选（用官方美术）"}
          </span>
        </div>
        <div className="field field--full">
          <span>说明书</span>
          <Button onClick={onPickDoc}>选 PDF…</Button>
          <span className="field__file">{docPath ? docPath.split(/[\\/]/).pop() : "没选"}</span>
        </div>
        {meta.campaign === "" && (
          <div className="field field--full">
            <span>主地图</span>
            <Choice value={meta.mainMap} options={mainMapOptions} onChange={(value) => onMeta({ ...meta, mainMap: value })} />
          </div>
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
