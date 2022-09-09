import { App } from 'vue';
import { createI18n } from 'vue-i18n';
import messages from '../i18n';

export default ({ app }: { app: App }) => {
  // Create I18n instance
  const i18n = createI18n({
    locale: navigator.language,
    fallbackLocale: 'en-US',
    globalInjection: true,
    messages,
  });

  // Tell app to use the I18n instance
  app.use(i18n);
};
