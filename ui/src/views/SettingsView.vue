<script setup lang="ts">
import { computed } from "vue";

import { MIYIN } from "../api/art";
import { useLauncher } from "../composables/useLauncher";

const { installation, chooseGameDirectory, reveal, isDesktop } = useLauncher();

const rows = computed(() => {
  const current = installation.value;
  return [
    { label: "安装根目录", value: current?.root },
    { label: "游戏启动器", value: current?.executable },
    { label: "版本切换器", value: current?.switcher },
    { label: "官方战役目录", value: current?.campaign_maps_root },
    { label: "自制战役目录", value: current?.custom_campaigns_root },
    { label: "模组目录", value: current?.mods_root },
    { label: "战役存档目录", value: current?.banks_root },
    { label: "用户文档目录", value: current?.documents_root },
  ].filter((row) => Boolean(row.value));
});
</script>

<template>
  <div class="page">
    <section class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">游戏目录</h3>
        <div class="panel__actions">
          <button class="btn btn-outline" type="button" @click="chooseGameDirectory">
            重新选择
          </button>
          <button
            v-if="installation"
            class="btn btn-text"
            type="button"
            @click="reveal(installation.root)"
          >
            打开目录
          </button>
        </div>
      </header>

      <dl v-if="rows.length" class="rows">
        <div v-for="row in rows" :key="row.label" class="row">
          <dt>{{ row.label }}</dt>
          <dd :title="row.value ?? ''">{{ row.value }}</dd>
        </div>
      </dl>
      <p v-else class="hint">尚未设置游戏目录。启动器会先尝试从注册表自动发现。</p>
    </section>

    <section class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">安装与安全</h3>
      </header>
      <ul class="notes">
        <li>
          <strong>安装前先核对。</strong>
          压缩包会先被完整读一遍：确认格式、解析元数据、检查是否存在越界路径条目
          （zip-slip）、统计解压后体积。这一步不写入任何文件。
        </li>
        <li>
          <strong>事务化落盘。</strong>
          内容先解压到暂存目录，校验通过后整体切换；切换失败会自动回滚，
          不会出现「旧版本已删、新版本没装上」的半途状态。
        </li>
        <li>
          <strong>删除范围严格受限。</strong>
          卸载只会删除战役自己的目录，绝不触碰父目录或官方战役地图。
        </li>
        <li>
          <strong>路径白名单。</strong>
          所有写操作都要求目标位于游戏目录或用户文档目录之内，并会解析符号链接后再校验一次。
        </li>
        <li>
          <strong>编码兼容。</strong>
          元数据优先按 UTF-8 解码，失败自动退回 GBK，中文战役包不会变成乱码。
        </li>
      </ul>
    </section>

    <section class="card panel about">
      <img class="about__art" :src="MIYIN.portrait" alt="弥音" />
      <div class="about__body">
        <header class="panel__head">
          <h3 class="panel__title">关于</h3>
          <span class="version">v0.1.0</span>
        </header>
        <p class="hint">
          弥音启动器（MiYin Launcher）—— 用 Rust 编写的星际争霸 II 战役与 Mod 管理器。
          美术风格参考 HMCL；战役格式兼容 CCM 与弥音标准包。
        </p>
        <p class="hint">
          当前运行模式：{{ isDesktop ? "桌面版（Tauri）" : "浏览器演示模式（数据为示例）" }}。
        </p>
      </div>
    </section>
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 18px;
  max-width: 900px;
  margin: 0 auto;
}

.panel {
  padding: 20px 24px 22px;
}

.panel__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 14px;
  margin-bottom: 14px;
}

.panel__title {
  margin: 0;
  font-size: 16px;
}

.panel__actions {
  display: flex;
  gap: 6px;
}

.rows {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0;
}

.row {
  display: grid;
  grid-template-columns: 128px 1fr;
  gap: 12px;
  padding: 9px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--outline) 35%, transparent);
}

.row:last-child {
  border-bottom: none;
}

.row dt {
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.row dd {
  margin: 0;
  font-size: 12.5px;
  word-break: break-all;
  font-family: "Cascadia Mono", "Consolas", monospace;
}

.notes {
  display: flex;
  flex-direction: column;
  gap: 11px;
  margin: 0;
  padding-left: 18px;
  font-size: 13px;
  line-height: 1.75;
  color: var(--on-surface-variant);
}

.notes strong {
  color: var(--on-surface);
}

.hint {
  margin: 0 0 8px;
  font-size: 13px;
  line-height: 1.8;
  color: var(--on-surface-variant);
}

.about {
  display: flex;
  gap: 20px;
  align-items: stretch;
}

.about__art {
  flex: 0 0 auto;
  width: 148px;
  align-self: stretch;
  min-height: 148px;
  object-fit: cover;
  object-position: center 34%;
  border-radius: var(--radius-md);
}

.about__body {
  flex: 1;
  min-width: 0;
}

.version {
  font-size: 12px;
  color: var(--on-surface-variant);
}
</style>
