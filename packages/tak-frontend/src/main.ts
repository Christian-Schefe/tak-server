import Main from '@/Main.vue';
import { VueQueryPlugin } from '@tanstack/vue-query';
import 'chartjs-adapter-date-fns';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import { useSettingsStore } from './features/settings';
import './main.css';
import router from './router';

import { createTakUI } from '@tak-ui-lib/components';
import { materialTheme } from '@tak-ui-lib/material';
import { useAuthStore } from './features/auth';

const app = createApp(Main);

app.use(createPinia());
app.use(router);
app.use(createTakUI(), {
  icons: {},
  theme: materialTheme,
});
app.use(VueQueryPlugin);

const settingsStore = useSettingsStore();
settingsStore.initializeSettings();

const authStore = useAuthStore();
router.beforeEach((to) => {
  const isAuthenticated = authStore.data?.userId !== undefined;
  if (!isAuthenticated && to.meta.noAuth !== true) {
    return { name: '/(auth)/login' };
  }
});
await Promise.all([authStore.maybeRefreshToken(), router.isReady()]);
app.mount('#app');
