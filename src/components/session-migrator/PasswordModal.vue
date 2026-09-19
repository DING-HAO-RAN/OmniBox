<template>
  <!-- 会话口令输入弹窗 (Fluent Design 拟态遮罩与卡片) -->
  <div
    v-if="visible"
    class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-md transition-all duration-200"
    @click.self="handleCancel"
  >
    <div
      class="w-full max-w-md rounded-3xl p-6 bg-[#161b22]/95 border border-white/10 shadow-2xl shadow-black/80 flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150"
    >
      <!-- 头部 -->
      <div class="flex items-center justify-between border-b border-white/5 pb-3">
        <div class="flex items-center gap-2.5">
          <div class="w-8 h-8 rounded-xl bg-purple-500/20 text-purple-400 border border-purple-500/30 flex items-center justify-center">
            <Lock class="w-4 h-4" />
          </div>
          <div>
            <h3 class="text-sm font-bold text-white">{{ title }}</h3>
            <p class="text-[11px] text-gray-400 mt-0.5">
              口令仅在内存中临时使用，不会保存至本地配置或日志。
            </p>
          </div>
        </div>
        <button
          @click="handleCancel"
          type="button"
          class="text-gray-400 hover:text-white p-1 rounded-lg hover:bg-white/5 transition-colors"
        >
          <X class="w-4 h-4" />
        </button>
      </div>

      <!-- 表单区域 -->
      <div class="space-y-3 pt-1">
        <div>
          <label class="block text-xs text-gray-300 font-medium mb-1.5">请输入口令：</label>
          <input
            ref="passwordInputRef"
            v-model="password"
            type="password"
            placeholder="输入解密/加密口令..."
            class="w-full h-10 px-3.5 rounded-xl bg-black/40 border border-white/10 focus:border-purple-500/70 focus:ring-2 focus:ring-purple-500/20 text-xs text-white placeholder-gray-500 outline-none transition-all"
            @keydown.enter="handleConfirm"
          />
        </div>

        <div v-if="confirm">
          <label class="block text-xs text-gray-300 font-medium mb-1.5">请再次输入确认口令：</label>
          <input
            v-model="passwordConfirmation"
            type="password"
            placeholder="再次输入口令..."
            class="w-full h-10 px-3.5 rounded-xl bg-black/40 border border-white/10 focus:border-purple-500/70 focus:ring-2 focus:ring-purple-500/20 text-xs text-white placeholder-gray-500 outline-none transition-all"
            @keydown.enter="handleConfirm"
          />
        </div>

        <p v-if="errorText" class="text-xs text-rose-400 flex items-center gap-1.5 pt-0.5">
          <AlertCircle class="w-3.5 h-3.5 flex-shrink-0" />
          <span>{{ errorText }}</span>
        </p>
      </div>

      <!-- 底部操作按钮 -->
      <div class="flex items-center justify-end gap-2.5 pt-2 border-t border-white/5">
        <button
          @click="handleCancel"
          type="button"
          class="px-4 py-2 rounded-xl text-xs font-medium text-gray-400 hover:text-white hover:bg-white/5 transition-colors"
        >
          取消
        </button>
        <button
          @click="handleConfirm"
          type="button"
          class="px-5 py-2 rounded-xl text-xs font-semibold text-white bg-gradient-to-r from-purple-600 to-indigo-600 hover:from-purple-500 hover:to-indigo-500 active:scale-95 shadow-lg shadow-purple-500/25 transition-all"
        >
          确定
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue';
import { Lock, X, AlertCircle } from 'lucide-vue-next';

const props = defineProps<{
  visible: boolean;
  title?: string;
  confirm?: boolean;
}>();

const emit = defineEmits<{
  (e: 'submit', password: string): void;
  (e: 'cancel'): void;
}>();

const password = ref('');
const passwordConfirmation = ref('');
const errorText = ref('');
const passwordInputRef = ref<HTMLInputElement | null>(null);

watch(
  () => props.visible,
  (val) => {
    if (val) {
      password.value = '';
      passwordConfirmation.value = '';
      errorText.value = '';
      nextTick(() => {
        passwordInputRef.value?.focus();
      });
    }
  }
);

function handleCancel() {
  emit('cancel');
}

function handleConfirm() {
  errorText.value = '';
  const pwd = password.value;
  if (!pwd) {
    errorText.value = '口令不能为空';
    return;
  }
  if (props.confirm) {
    if (pwd !== passwordConfirmation.value) {
      errorText.value = '两次输入的口令不一致，请重新检查';
      return;
    }
  }
  emit('submit', pwd);
}
</script>
