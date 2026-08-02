import { createApp } from 'vue';
import Main from './Main.vue';
import router from './router';
import './style.css';

import { createTakUI } from '@tak-ui-lib/components';
import { LuComponent, LuHome, LuMoon, LuPaintbrush, LuSun } from 'vue-icons-plus/lu';

const app = createApp(Main);
app.use(router);
app.use(createTakUI(), {
  icons: {
    home: LuHome,
    component: LuComponent,
    theme: LuPaintbrush,
    darkMode: LuMoon,
    lightMode: LuSun,
  },
});
app.mount('#app');
