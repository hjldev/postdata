<script setup lang="ts">
import { Plus, Trash2, FileUp } from "lucide-vue-next";
import { open } from "@tauri-apps/plugin-dialog";
import { pair, type Pair } from "../types";
const props = defineProps<{
  rows: Pair[];
  files?: boolean;
  disabled?: boolean;
}>();
const emit = defineEmits<{ change: []; error: [message: string] }>();
function add() {
  props.rows.push(pair());
  emit("change");
}
function remove(i: number) {
  props.rows.splice(i, 1);
  emit("change");
}
async function choose(row: Pair) {
  try {
    const path = await open({ multiple: false, directory: false });
    if (path) {
      row.value = path;
      emit("change");
    }
  } catch (e) {
    emit("error", String(e));
  }
}
</script>
<template>
  <div class="pair-editor">
    <div class="pair-row pair-heading">
      <span></span><span>参数名 / KEY</span><span>参数值 / VALUE</span
      ><span></span>
    </div>
    <div
      v-for="(row, i) in rows"
      :key="row.id"
      class="pair-row"
      :class="{ muted: !row.enabled }"
    >
      <input
        v-model="row.enabled"
        type="checkbox"
        :disabled="disabled"
        aria-label="启用此行"
        @change="emit('change')"
      />
      <div class="key-cell">
        <input
          v-model="row.key"
          :disabled="disabled"
          placeholder="参数名"
          aria-label="参数名"
          @input="emit('change')"
        /><select
          v-if="files"
          v-model="row.kind"
          :disabled="disabled"
          aria-label="字段类型"
          @change="
            row.value = '';
            emit('change');
          "
        >
          <option value="text">文本</option>
          <option value="file">文件</option>
        </select>
      </div>
      <div class="value-cell">
        <input
          v-model="row.value"
          :disabled="disabled"
          :readonly="row.kind === 'file' && files"
          :placeholder="
            row.kind === 'file' && files ? '选择要上传的文件' : '参数值'
          "
          aria-label="参数值"
          @input="emit('change')"
        /><button
          v-if="row.kind === 'file' && files"
          class="icon-button"
          :disabled="disabled"
          title="选择文件"
          @click="choose(row)"
        >
          <FileUp :size="15" />
        </button>
      </div>
      <button
        class="icon-button delete-row"
        :disabled="disabled"
        title="删除此行"
        @click="remove(i)"
      >
        <Trash2 :size="14" />
      </button>
    </div>
    <button class="add-row" :disabled="disabled" @click="add">
      <Plus :size="14" /> 添加一行
    </button>
    <p v-if="!rows.length" class="table-hint">
      添加参数来配置你的请求，未勾选的行不会发送。
    </p>
  </div>
</template>
