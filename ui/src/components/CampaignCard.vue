<script setup lang="ts">
import { computed, ref, watch } from "vue";

import { api } from "../api/bridge";
import { campaignArt, campaignTypeName, formatBytes } from "../api/art";
import type { Campaign } from "../api/types";

const props = defineProps<{ campaign: Campaign }>();
const emit = defineEmits<{ uninstall: [id: string]; reveal: [path: string] }>();

/** 包内自带封面（data URL），没有则退回资料片官方美术。 */
const customCover = ref<string | null>(null);

const cover = computed(() => customCover.value ?? campaignArt(props.campaign.campaign_type));

const healthLabel = computed(() => {
  switch (props.campaign.health) {
    case "ok":
      return "正常";
    case "warning":
      return "需注意";
    default:
      return "无法运行";
  }
});

watch(
  () => props.campaign.path,
  async (path) => {
    customCover.value = null;
    try {
      customCover.value = await api.campaignCover(path);
    } catch {
      customCover.value = null;
    }
  },
  { immediate: true },
);

const coverStyle = computed(() =>
  cover.value ? { backgroundImage: "url(" + cover.value + ")" } : undefined,
);
</script>

<template>
  <article class="campaign" :class="'campaign--' + campaign.health">
    <div class="campaign__cover" :style="coverStyle">
      <div class="campaign__scrim"></div>
      <span class="campaign__health" :class="'health--' + campaign.health">{{ healthLabel }}</span>
      <span v-if="campaign.enabled" class="campaign__enabled">已启用</span>
    </div>

    <div class="campaign__body">
      <h4 class="campaign__name" :title="campaign.name">{{ campaign.name }}</h4>

      <p class="campaign__meta">
        <span>{{ campaignTypeName(campaign.campaign_type) }}</span>
        <span v-if="campaign.author">· {{ campaign.author }}</span>
        <span v-if="campaign.version">· v{{ campaign.version }}</span>
      </p>

      <p class="campaign__stats">
        {{ campaign.map_count ?? 0 }} 张地图 · {{ formatBytes(campaign.size_bytes) }}
      </p>

      <ul v-if="campaign.issues.length" class="campaign__issues">
        <li
          v-for="issue in campaign.issues"
          :key="issue.code"
          class="issue"
          :class="'issue--' + issue.level"
          :title="issue.hint ?? ''"
        >
          {{ issue.message }}
        </li>
      </ul>

      <div class="campaign__actions">
        <button class="btn btn-text" type="button" @click="emit('reveal', campaign.path)">
          打开目录
        </button>
        <button
          class="btn btn-text campaign__danger"
          type="button"
          @click="emit('uninstall', campaign.id)"
        >
          卸载
        </button>
      </div>
    </div>
  </article>
</template>

<style scoped>
.campaign {
  display: flex;
  flex-direction: column;
  background: var(--surface-1);
  border-radius: var(--radius-lg);
  overflow: hidden;
  box-shadow: var(--shadow-2);
  border: 1px solid color-mix(in srgb, var(--outline) 40%, transparent);
  transition: transform var(--duration) var(--ease), box-shadow var(--duration) var(--ease);
}

.campaign:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-3);
}

.campaign--broken {
  border-color: color-mix(in srgb, var(--danger) 45%, transparent);
}

.campaign__cover {
  position: relative;
  aspect-ratio: 16 / 8.4;
  background-color: #1b1630;
  background-size: cover;
  background-position: center;
}

.campaign__scrim {
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(12, 9, 26, 0) 40%, rgba(12, 9, 26, 0.72) 100%);
}

.campaign__health,
.campaign__enabled {
  position: absolute;
  top: 10px;
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  font-size: 11.5px;
  font-weight: 600;
  backdrop-filter: blur(6px);
}

.campaign__health {
  left: 10px;
}

.health--ok {
  background: rgba(46, 125, 50, 0.86);
  color: #fff;
}

.health--warning {
  background: rgba(194, 120, 0, 0.9);
  color: #fff;
}

.health--broken {
  background: rgba(186, 26, 26, 0.9);
  color: #fff;
}

.campaign__enabled {
  right: 10px;
  background: rgba(103, 80, 164, 0.9);
  color: #fff;
}

.campaign__body {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 14px 16px 10px;
}

.campaign__name {
  margin: 0;
  font-size: 15.5px;
  font-weight: 700;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.campaign__meta,
.campaign__stats {
  margin: 0;
  font-size: 12.5px;
  color: var(--on-surface-variant);
}

.campaign__issues {
  margin: 2px 0 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.issue {
  font-size: 12px;
  padding: 5px 9px;
  border-radius: var(--radius-xs);
  line-height: 1.4;
}

.issue--warning {
  background: var(--warning-soft);
  color: var(--warning);
}

.issue--broken {
  background: var(--danger-soft);
  color: var(--danger);
}

.issue--ok {
  background: var(--success-soft);
  color: var(--success);
}

.campaign__actions {
  display: flex;
  justify-content: flex-end;
  gap: 2px;
  margin-top: 4px;
  padding-top: 8px;
  border-top: 1px solid color-mix(in srgb, var(--outline) 45%, transparent);
}

.campaign__danger {
  color: var(--danger);
}
</style>
