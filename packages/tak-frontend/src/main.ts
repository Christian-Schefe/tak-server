import App from '@/App.vue';
import Aura from '@primeuix/themes/aura';
import { VueQueryPlugin } from '@tanstack/vue-query';
import 'chartjs-adapter-date-fns';
import { createPinia } from 'pinia';
import PrimeVue from 'primevue/config';
import Ripple from 'primevue/ripple';
import { createApp } from 'vue';
import './main.css';
import router from './router';
import { useSettingsStore } from './features/settings';

const app = createApp(App);

app.use(createPinia());
app.use(router);
app.use(PrimeVue, {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: '.dark-mode',
    },
  },
  ripple: true,
});
app.use(VueQueryPlugin);

app.directive('ripple', Ripple);

const settingsStore = useSettingsStore();
settingsStore.initializeSettings();

app.mount('#app');
