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

app.mount('#app');
