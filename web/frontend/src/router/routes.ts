import { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('layouts/MainLayout.vue'),
  },
  {
    path: '/place/:osm_id',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/place/:osm_id',
        props: true,
        component: () => import('pages/PlacePage.vue'),
      },
    ],
  },
  {
    path: '/pin/:long/:lat',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/pin/:long/:lat',
        props: true,
        component: () => import('pages/DroppedPinPage.vue'),
      },
    ],
  },

  // Always leave this as last one,
  // but you can also remove it
  {
    path: '/:catchAll(.*)*',
    component: () => import('pages/ErrorNotFound.vue'),
  },
];

export default routes;
