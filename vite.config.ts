import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],

  // 防止 Vite 清除控制台 Rust 输出
  clearScreen: false,

  server: {
    // Tauri 开发服务器固定端口
    port: 1420,
    strictPort: true,
    watch: {
      // 避免监听 Rust 源码目录，防止多余热重载
      ignored: ["**/src-tauri/**"],
    },
  },
});
