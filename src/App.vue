<script setup lang="ts">
import { onMounted, ref } from "vue";
import { Channel, invoke } from "@tauri-apps/api/core";

type KeystrokeStreamEvent =
  | { kind: "key"; data: string }
  | { kind: "error"; data: string };

const keys = ref<string[]>([]);
const status = ref("Starting listener…");

// Keep a reference for the lifetime of the app: Rust holds the channel id and
// pushes every event through the `onmessage` handler below.
const keystrokes = new Channel<KeystrokeStreamEvent>();

keystrokes.onmessage = (event) => {
  if (event.kind === "key") {
    status.value = "Listening";
    keys.value = [...keys.value.slice(-29), event.data];
  } else {
    status.value = event.data;
  }
};

onMounted(async () => {
  try {
    await invoke("start_keystroke_stream", { channel: keystrokes });
    if (status.value === "Starting listener…") {
      status.value = "Listening";
    }
  } catch (error) {
    status.value = `Could not start listener: ${error}`;
  }
});
</script>

<template>
  <main class="container">
    <h1>Keytastrophe</h1>
    <p class="status">{{ status }}</p>
    <p class="hint">Type anywhere — keystrokes appear here.</p>
    <div class="keys" aria-live="polite">
      <span v-for="(key, index) in keys" :key="index" class="key">{{ key }}</span>
    </div>
  </main>
</template>

<style scoped>
h1 {
  margin: 0 0 0.5rem;
  font-size: 1.6rem;
  letter-spacing: -0.02em;
}

.status {
  margin-bottom: 0.25rem;
  font-size: 0.95rem;
  opacity: 0.8;
}

.hint {
  margin-top: 0;
  font-size: 0.85rem;
  opacity: 0.55;
}

.keys {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 8px;
  max-width: 640px;
  min-height: 2.5rem;
  margin: 1.5rem auto 0;
  padding: 0 1.5rem;
}

.key {
  padding: 0.35em 0.7em;
  border: 1px solid rgba(128, 128, 128, 0.35);
  border-radius: 6px;
  background: rgba(128, 128, 128, 0.1);
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.9rem;
}

.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}
</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}
</style>
