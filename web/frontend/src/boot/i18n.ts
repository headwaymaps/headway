import { App } from 'vue';
import { createI18n } from 'vue-i18n';
import messages from '../i18n';

export default ({ app }: { app: App }) => {
  app.use(createI18nForApp());
};

export const createI18nForApp = () => createI18n({
  locale: navigator.language,
  fallbackLocale: 'en-US',
  globalInjection: true,
  messages,
});