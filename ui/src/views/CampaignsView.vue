<script setup lang="ts">
import { computed, ref } from "vue";

import { MIYIN } from "../api/art";
import CampaignCard from "../components/CampaignCard.vue";
import InstallDialog from "../components/InstallDialog.vue";
import { useLauncher } from "../composables/useLauncher";

const emit = defineEmits<{ "open-settings": [] }>();

const { installation, campaigns, loading, launch, reveal, uninstall, refresh, chooseGameDirectory } =
  useLauncher();

const showInstall = ref(false);

const regionLabel = computed(() => {
  const branch = installation.value?.branch;
  if (!branch) return "未知区域";
  return branch.toLowerCase() === "cn" ? "国服" : branch.toUpperCase();
});

const healthyCount = computed(() => campaigns.value.filter((item) => item.health === "ok").length);
</script>

<template>
  <div class="page">
    <!-- 尚未设置游戏目录 -->
    <section v-if="!installation" class="empty card">
      <img class="empty__art" :src="MIYIN.cry" alt="" />
      <h2 class="empty__title">还没有找到星际争霸 II</h2>
      <p class="empty__text">
        启动器会先读取注册表里的安装位置。如果没有找到，请手动指定游戏根目录
        （即包含 <code>StarCraft II.exe</code> 的那一层）。
      </p>
      <div class="empty__actions">
        <button class="btn btn-primary btn-lg" type="button" @click="chooseGameDirectory">
          选择游戏目录
        </button>
        <button class="btn btn-outline" type="button" @click="emit('open-settings')">
          前往设置
        </button>
      </div>
    </section>

    <template v-else>
      <!-- 游戏状态 + 看板娘 -->
      <section class="hero card">
        <div class="hero__body">
          <p class="hero__eyebrow">星际争霸 II · {{ regionLabel }}</p>
          <h2 class="hero__title">{{ installation.version ?? "版本未知" }}</h2>
          <p class="hero__path" :title="installation.root">{{ installation.root }}</p>

          <div class="hero__actions">
            <button class="btn btn-primary btn-lg" type="button" @click="launch">开始游戏</button>
            <button class="btn btn-outline" type="button" @click="reveal(installation.root)">
              打开游戏目录
            </button>
          </div>

          <dl class="hero__stats">
            <div class="stat">
              <dt>已安装战役</dt>
              <dd>{{ campaigns.length }}</dd>
            </div>
            <div class="stat">
              <dt>状态正常</dt>
              <dd>{{ healthyCount }}</dd>
            </div>
            <div class="stat">
              <dt>构建号</dt>
              <dd>{{ installation.build ?? "—" }}</dd>
            </div>
          </dl>
        </div>

        <div class="hero__kanban" aria-hidden="true">
          <img :src="MIYIN.wink" alt="" />
        </div>
      </section>

      <!-- 战役列表 -->
      <div class="section-head">
        <h3 class="section-head__title">
          我的战役
          <span class="section-head__count">{{ campaigns.length }}</span>
        </h3>
        <div class="section-head__actions">
          <button class="btn btn-text" type="button" :disabled="loading" @click="refresh">
            {{ loading ? "扫描中…" : "重新扫描" }}
          </button>
          <button class="btn btn-tonal" type="button" @click="showInstall = true">
            ＋ 安装战役包
          </button>
        </div>
      </div>

      <div v-if="campaigns.length" class="grid">
        <CampaignCard
          v-for="campaign in campaigns"
          :key="campaign.id"
          :campaign="campaign"
          @reveal="reveal"
          @uninstall="uninstall"
        />
      </div>

      <section v-else class="empty empty--compact card">
        <img class="empty__art empty__art--small" :src="MIYIN.cry" alt="" />
        <h2 class="empty__title">还没有安装自制战役</h2>
        <p class="empty__text">
          支持 CCM 战役包（内含 <code>metadata.txt</code>）与弥音标准包（根目录含
          <code>metadata.json</code>）。安装前会先做一次完整核对，不会直接往游戏目录里写东西。
        </p>
        <div class="empty__actions">
          <button class="btn btn-primary" type="button" @click="showInstall = true">
            安装第一个战役
          </button>
        </div>
      </section>
    </template>

    <InstallDialog v-if="showInstall" @close="showInstall = false" />
  </div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-width: var(--content-max);
  margin: 0 auto;
}

/* ---------- 顶部看板 ---------- */

.hero {
  position: relative;
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  overflow: hidden;
  padding: 24px 28px;
  background:
    radial-gradient(360px 280px at 86% 74%, rgba(178, 150, 255, 0.42), transparent 72%),
    linear-gradient(118deg, #ffffff 0%, #fdfaff 58%, #f7f0ff 100%);
  border: 1px solid color-mix(in srgb, var(--outline) 42%, transparent);
}

.hero__body {
  position: relative;
  z-index: 2;
  max-width: 520px;
}

.hero__eyebrow {
  margin: 0 0 6px;
  font-size: 12.5px;
  letter-spacing: 1.4px;
  text-transform: uppercase;
  color: var(--accent);
  font-weight: 700;
}

.hero__title {
  margin: 0;
  font-size: 30px;
  font-weight: 700;
  letter-spacing: 0.4px;
  color: var(--on-surface);
}

.hero__path {
  margin: 8px 0 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
  word-break: break-all;
}

.hero__actions {
  display: flex;
  gap: 10px;
  margin-top: 18px;
}

.hero__stats {
  display: flex;
  gap: 26px;
  margin: 18px 0 0;
  padding-top: 14px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.stat dt {
  font-size: 12px;
  color: var(--on-surface-variant);
}

.stat dd {
  margin: 2px 0 0;
  font-size: 20px;
  font-weight: 700;
  color: var(--accent);
}

/* 看板娘：立绘自带白底，用径向遮罩把方形边缘化开 */
.hero__kanban {
  position: relative;
  z-index: 1;
  flex: 0 0 auto;
  width: 218px;
  margin: -24px -10px -24px 0;
  pointer-events: none;
}

.hero__kanban img {
  display: block;
  width: 100%;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
  mask-image: radial-gradient(circle at 50% 52%, #000 58%, transparent 78%);
}

/* ---------- 列表 ---------- */

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 4px;
}

.section-head__title {
  display: flex;
  align-items: center;
  gap: 10px;
  margin: 0;
  font-size: 17px;
  font-weight: 700;
  color: #fff;
  text-shadow: 0 1px 6px rgba(0, 0, 0, 0.55);
}

.section-head__count {
  padding: 1px 9px;
  border-radius: var(--radius-pill);
  background: rgba(255, 255, 255, 0.22);
  font-size: 12.5px;
}

.section-head__actions {
  display: flex;
  gap: 8px;
}

/* 这一行浮在深色背景上，默认的紫色文字按钮几乎看不见 */
.section-head__actions .btn-text {
  color: #fff;
  background: rgba(255, 255, 255, 0.16);
}

.section-head__actions .btn-text:not(:disabled):hover {
  background: rgba(255, 255, 255, 0.26);
}

.grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(272px, 1fr));
  gap: 18px;
}

/* ---------- 空状态 ---------- */

.empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  gap: 10px;
  padding: 40px 32px 46px;
}

.empty--compact {
  padding: 30px 32px 38px;
}

.empty__art {
  width: 176px;
  height: 176px;
  object-fit: contain;
  margin-bottom: -6px;
  -webkit-mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
  mask-image: radial-gradient(circle at 50% 52%, #000 62%, transparent 82%);
}

.empty__art--small {
  width: 132px;
  height: 132px;
}

.empty__title {
  margin: 4px 0 0;
  font-size: 19px;
}

.empty__text {
  margin: 0;
  max-width: 560px;
  color: var(--on-surface-variant);
  font-size: 13.5px;
  line-height: 1.7;
}

.empty__text code {
  padding: 1px 6px;
  border-radius: var(--radius-xs);
  background: var(--surface-3);
  font-size: 12.5px;
}

.empty__actions {
  display: flex;
  gap: 10px;
  margin-top: 12px;
}
</style>
