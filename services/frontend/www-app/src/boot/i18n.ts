import { App } from 'vue';
import { createI18n } from 'vue-i18n';
import messages from '../i18n';

export default ({ app }: { app: App }) => {
  app.use(createI18nForApp());
};

export const createI18nForApp = () => {
  let locale;
  if (process.env.NODE_ENV == 'test') {
    locale = 'en-US';
  } else {
    locale = navigator.language;
  }
  return createI18n({
    locale,
    fallbackLocale: 'en-US',
    globalInjection: true,
    messages,
  });
};
