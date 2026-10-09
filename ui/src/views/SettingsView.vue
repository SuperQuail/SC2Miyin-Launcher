<script setup lang="ts">
import { computed, onMounted } from "vue";

import { MIYIN } from "../api/art";
import { useLauncher } from "../composables/useLauncher";
import ToolsPanel from "../components/ToolsPanel.vue";
import UpdatePanel from "../components/UpdatePanel.vue";

const openDevPage = () => { location.href = "/ide.html"; };

const {
  installation,
  libraryRoot,
  chooseGameDirectory,
  reveal,
  isDesktop,
  installedCampaigns,
  refreshInstalled,
  collectCampaign,
  saves,
  saveBackups,
  refreshSaves,
  backupSaves,
  restoreSaves,
} = useLauncher();

onMounted(() => {
  void refreshSaves();
  void refreshInstalled();
});

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
        <h3 class="panel__title">战役库</h3>
        <button class="btn btn-text" type="button" @click="reveal(libraryRoot)">打开库目录</button>
      </header>
      <p class="path">{{ libraryRoot || "（未初始化）" }}</p>
      <p class="hint">
        导入的每个玩家版本都完整保存在这里（软件同级的 data 目录），与游戏目录解耦。
        同一个战役可以并存多个版本，切换时只启用选中的那一份。
      </p>
    </section>

    <section class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">安装与安全</h3>
      </header>
      <ul class="notes">
        <li>
          <strong>导入前先看一遍。</strong>
          打开压缩包时只做检查 —— 确认里面是不是战役包、有没有危险的文件路径、
          解开后有多大。这一步不写任何文件。
        </li>
        <li>
          <strong>出错不会留下半成品。</strong>
          内容先解到临时目录，确认完整了才正式入库；中途失败会自动清理干净。
        </li>
        <li>
          <strong>切换只动自己放的文件。</strong>
          启用某个版本时，放进去的每个文件都会记账；切回原版时只删这些文件。
          遇到同名的官方文件会先备份，切回时原样还原 —— 不会去删官方目录。
        </li>
        <li>
          <strong>只往游戏目录里写。</strong>
          所有写入都限制在游戏的 Maps / Mods 等目录之内，不会碰别的地方。
        </li>
        <li>
          <strong>中文不会乱码。</strong>
          包里的说明文字会自动识别编码，简繁中文的战役包都能正常显示。
        </li>
      </ul>
    </section>

    <section v-if="installedCampaigns.length" class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">游戏目录里已经装着的</h3>
        <span class="version">{{ installedCampaigns.length }} 个</span>
      </header>
      <p class="hint">
        这些是直接扔进 <code>Maps/CustomCampaigns</code> 的战役，还没进库 —— 收编之后就能用启动器管版本了。
        <strong>收编不会删原目录。</strong>
      </p>
      <ul class="plist">
        <li v-for="item in installedCampaigns" :key="item.dir" class="pitem">
          <span class="pitem__name">{{ item.name }}</span>
          <span class="tag">{{ item.files }} 个文件 · {{ (item.bytes / 1024 / 1024).toFixed(1) }} MB</span>
          <button class="btn btn-text" type="button" @click="collectCampaign(item.dir)">收进库</button>
        </li>
      </ul>
    </section>

    <section class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">存档</h3>
        <span class="version">我的文档 / StarCraft II / Banks</span>
      </header>
      <p class="hint" v-if="saves?.missing">还没有存档 —— 玩过一关之后这里才会有东西。</p>
      <p class="hint" v-else-if="saves">
        现在有 {{ saves.files.length }} 个存档文件，共 {{ (saves.bytes / 1024).toFixed(0) }} KB。
      </p>
      <p class="about__actions">
        <button class="btn btn-tonal" type="button" :disabled="!saves || saves.missing" @click="backupSaves('手动')">
          备份这份存档
        </button>
      </p>
      <template v-if="saveBackups.length">
        <p class="hint">已备份 {{ saveBackups.length }} 份。还原前会先把现在这份另存一份。</p>
        <ul class="plist">
          <li v-for="item in saveBackups" :key="item.name" class="pitem">
            <span class="pitem__name">{{ item.label }}</span>
            <span class="tag">{{ item.files }} 个文件</span>
            <button class="btn btn-text" type="button" @click="restoreSaves(item.name)">还原</button>
          </li>
        </ul>
      </template>
    </section>

    <section class="card panel">
      <header class="panel__head">
        <h3 class="panel__title">开发者页</h3>
        <span class="version">实验</span>
      </header>
      <p class="hint">
        给包作者的工作台：扫游戏目录、勾选要打进包的内容、按行读文本文件。
        地图和模组这类二进制会明确告诉你打不开、可以用什么打开。
      </p>
      <p class="hint">版本管理（提交 / 回滚 / 日志）还没接，界面里那一块标着「示例」。</p>
      <p class="about__actions">
        <button class="btn btn-tonal" type="button" @click="openDevPage">打开开发者页</button>
      </p>
    </section>

    <ToolsPanel />
    <UpdatePanel />

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

.path {
  margin: 0 0 10px;
  font-size: 12.5px;
  word-break: break-all;
  font-family: "Cascadia Mono", "Consolas", monospace;
  color: var(--on-surface);
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
