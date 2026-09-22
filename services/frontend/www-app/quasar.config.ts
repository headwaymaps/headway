/* eslint-env node */

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Plugin } from 'vite';

/*
 * This file runs in a Node context (it's NOT transpiled by Babel), so use only
 * the ES6 features that are supported by your Node version. https://node.green/
 */

// Configuration for your app
// https://v2.quasar.dev/quasar-cli-vite/quasar-config-js

import { defineConfig } from '#q-app/wrappers';

const HEADWAY_HOST = 'https://maps.earth';

// Serve the checked-out style locally, while its tiles, sprites, and fonts continue through the
// /tileserver proxy. This makes it possible to iterate on a style against upstream world data.
//   HEADWAY_LOCAL_STYLE=1 yarn dev
const LOCAL_STYLE_ENABLED = process.env.HEADWAY_LOCAL_STYLE === '1';
const LOCAL_STYLE_PATH = resolve(
  __dirname,
  '../../tileserver/assets/styles/basic-v3.json',
);

// Sprites and fonts come from a martin serving ./services/tileserver/assets, so new
// sprites show up without publishing them, while tiles still come from upstream.
//   HEADWAY_LOCAL_STYLE=1 HEADWAY_LOCAL_ASSETS=http://localhost:8095 yarn dev
const LOCAL_ASSETS_HOST = process.env.HEADWAY_LOCAL_ASSETS;

const localStylePlugin: Plugin = {
  name: 'headway-local-style',
  configureServer(server) {
    if (!LOCAL_STYLE_ENABLED) {
      return;
    }

    server.middlewares.use((request, response, next) => {
      if (request.url?.split('?')[0] !== '/local-style/basic-v3.json') {
        next();
        return;
      }

      // maplibre rejects a relative sprite URL outright, so serve an absolute one.
      const origin = `http://${request.headers.host}`;

      const style = JSON.parse(readFileSync(LOCAL_STYLE_PATH, 'utf8'));
      for (const source of Object.values(style.sources) as Array<{
        url?: string;
      }>) {
        if (source.url?.startsWith('/')) {
          source.url = `/tileserver${source.url}`;
        }
      }
      const assetPrefix = LOCAL_ASSETS_HOST ? '/local-assets' : '/tileserver';
      style.sprite = `${origin}${assetPrefix}${style.sprite}`;
      style.glyphs = `${origin}${assetPrefix}${style.glyphs}`;

      response.setHeader('Content-Type', 'application/json');
      response.setHeader('Cache-Control', 'no-store');
      response.end(JSON.stringify(style));
    });

    server.watcher.add(LOCAL_STYLE_PATH);
    server.watcher.on('change', (path) => {
      if (path === LOCAL_STYLE_PATH) {
        server.ws.send({ type: 'full-reload' });
      }
    });
  },
};

export default defineConfig((/* ctx */) => {
  return {
    eslint: {
      // fix: true,
      // include = [],
      // exclude = [],
      // rawOptions = {},
      warnings: true,
      errors: true,
    },

    // https://v2.quasar.dev/quasar-cli-vite/prefetch-feature
    // preFetch: true,

    // app boot file (/src/boot)
    // --> boot files are part of "main.js"
    // https://v2.quasar.dev/quasar-cli-vite/boot-files
    boot: ['config', 'i18n'],

    // https://v2.quasar.dev/quasar-cli-vite/quasar-config-js#css
    css: ['app.scss'],

    // https://github.com/quasarframework/quasar/tree/dev/extras
    extras: [
      // 'ionicons-v4',
      // 'mdi-v5',
      // 'fontawesome-v6',
      // 'eva-icons',
      // 'themify',
      // 'line-awesome',
      // 'roboto-font-latin-ext', // this or either 'roboto-font', NEVER both!

      'roboto-font', // optional, you are not bound to it
      'material-icons', // optional, you are not bound to it
    ],

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/quasar-config-js#build
    build: {
      env: {
        HEADWAY_LOCAL_STYLE: String(LOCAL_STYLE_ENABLED),
      },
      target: {
        browser: ['es2019', 'edge88', 'firefox78', 'chrome87', 'safari13.1'],
        node: 'node16',
      },

      typescript: {
        strict: true,
        vueShim: true,
        // extendsTsConfig (tsConfig) {}
      },

      vueRouterMode: 'history', // available values: 'hash', 'history'
      // Dev: Use this where we don't have mod_rewrite, otherwise refreshing page with path 404's
      // vueRouterMode: 'hash',

      // vueRouterBase,
      // vueDevtools,
      // vueOptionsAPI: false,

      // rebuildCache: true, // rebuilds Vite/linter/etc cache on startup

      // publicPath: '/',
      // analyze: true,
      // env: {},
      // rawDefine: {}
      // ignorePublicFolder: true,
      // minify: false,
      // polyfillModulePreload: true,
      // distDir

      // extendViteConf (viteConf) {},
      // viteVuePluginOptions: {},

      vitePlugins: [
        localStylePlugin,
        [
          'vite-plugin-checker',
          {
            vueTsc: true,
            eslint: {
              lintCommand:
                'eslint -c ./eslint.config.mjs "./src/**/*.{ts,js,mjs,cjs,vue}"',
              useFlatConfig: true,
            },
          },
          { server: false },
        ],
      ],
    },

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/quasar-config-js#devServer
    devServer: {
      // Keep port fallback and browser opening on the same loopback interface.
      // Otherwise a service bound to 127.0.0.1 (for example Valhalla) can occupy
      // a port that Vite only probes over IPv6, and the opened localhost URL
      // reaches that other service instead of this dev server.
      host: '127.0.0.1',
      // https: true
      open: true, // opens browser window automatically
      proxy: {
        ...(LOCAL_ASSETS_HOST && {
          '/local-assets': {
            changeOrigin: true,
            target: LOCAL_ASSETS_HOST,
            rewrite: (path: string) =>
              path.replace(/^\/local-assets/, '/tileserver'),
          },
        }),
        '/tileserver': {
          // martin tileserver needs to receive headers in order to expand relative paths to the right
          // protocol+host
          xfwd: true,
          changeOrigin: true,
          target: HEADWAY_HOST,
          // target: 'http://localhost:8000',
          // rewrite: (path) => path.replace(/^\/tileserver/, ''),
        },
        '/pelias': {
          changeOrigin: true,
          target: HEADWAY_HOST,
          // target: 'http://0.0.0.0:4000',
          // rewrite: (path) => path.replace(/^\/pelias/, ''),
        },
        '/travelmux': {
          changeOrigin: true,
          target: HEADWAY_HOST,
          // target: 'http://0.0.0.0:8000',
          // rewrite: (path) => path.replace(/^\/travelmux/, ''),
        },
        '/transit-zoner': {
          changeOrigin: true,
          target: HEADWAY_HOST,
          // target: 'http://127.0.0.1:8420',
          // rewrite: (path) => path.replace(/^\/transit-zoner/, ''),
        },
      },
    },

    // https://v2.quasar.dev/quasar-cli-vite/quasar-config-js#framework
    framework: {
      config: {},

      // iconSet: 'material-icons', // Quasar icon set
      // lang: 'en-US', // Quasar language pack

      // For special cases outside of where the auto-import strategy can have an impact
      // (like functional components as one of the examples),
      // you can manually specify Quasar components/directives to be available everywhere:
      //
      // components: [],
      // directives: [],

      // Quasar plugins
      plugins: [],
    },

    // animations: 'all', // --- includes all animations
    // https://v2.quasar.dev/options/animations
    animations: [],

    // https://v2.quasar.dev/quasar-cli-vite/quasar-config-js#sourcefiles
    // sourceFiles: {
    //   rootComponent: 'src/App.vue',
    //   router: 'src/router/index',
    //   store: 'src/store/index',
    //   registerServiceWorker: 'src-pwa/register-service-worker',
    //   serviceWorker: 'src-pwa/custom-service-worker',
    //   pwaManifestFile: 'src-pwa/manifest.json',
    //   electronMain: 'src-electron/electron-main',
    //   electronPreload: 'src-electron/electron-preload'
    // },

    // https://v2.quasar.dev/quasar-cli-vite/developing-ssr/configuring-ssr
    ssr: {
      // ssrPwaHtmlFilename: 'offline.html', // do NOT use index.html as name!
      // will mess up SSR

      // extendSSRWebserverConf (esbuildConf) {},
      // extendPackageJson (json) {},

      pwa: false,

      // manualStoreHydration: true,
      // manualPostHydrationTrigger: true,

      prodPort: 3000, // The default port that the production server should use
      // (gets superseded if process.env.PORT is specified at runtime)

      middlewares: [
        'render', // keep this as last one
      ],
    },

    // https://v2.quasar.dev/quasar-cli-vite/developing-pwa/configuring-pwa
    pwa: {
      workboxMode: 'generateSW', // or 'injectManifest'
      injectPwaMetaTags: true,
      swFilename: 'sw.js',
      manifestFilename: 'manifest.json',
      useCredentialsForManifestTag: false,
      // extendGenerateSWOptions (cfg) {}
      // extendInjectManifestOptions (cfg) {},
      // extendManifestJson (json) {}
      // extendPWACustomSWConf (esbuildConf) {}
    },

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/developing-cordova-apps/configuring-cordova
    cordova: {
      // noIosLegacyBuildFlag: true, // uncomment only if you know what you are doing
    },

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/developing-capacitor-apps/configuring-capacitor
    capacitor: {
      hideSplashscreen: true,
    },

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/developing-electron-apps/configuring-electron
    electron: {
      // extendElectronMainConf (esbuildConf)
      // extendElectronPreloadConf (esbuildConf)

      inspectPort: 5858,

      bundler: 'packager', // 'packager' or 'builder'

      packager: {
        // https://github.com/electron-userland/electron-packager/blob/master/docs/api.md#options
        // OS X / Mac App Store
        // appBundleId: '',
        // appCategoryType: '',
        // osxSign: '',
        // protocol: 'myapp://path',
        // Windows only
        // win32metadata: { ... }
      },

      builder: {
        // https://www.electron.build/configuration/configuration

        appId: 'headway-frontend',
      },
    },

    // Full list of options: https://v2.quasar.dev/quasar-cli-vite/developing-browser-extensions/configuring-bex
    bex: {
      contentScripts: ['my-content-script'],

      // extendBexScriptsConf (esbuildConf) {}
      // extendBexManifestJson (json) {}
    },
  };
});
