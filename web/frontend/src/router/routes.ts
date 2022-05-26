import { RouteRecordRaw } from 'vue-router';

const routes: RouteRecordRaw[] = [
  {
    path: '/',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/',
        props: true,
        component: () => import('pages/BaseMapPage.vue'),
      },
    ],
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
  {
    path: '/directions/:mode/:to',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/directions/:mode/:to',
        props: true,
        component: () => import('pages/DirectionsPage.vue'),
      },
    ],
  },
  {
    path: '/directions/:mode/:to/:from',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/directions/:mode/:to/:from',
        props: true,
        component: () => import('pages/DirectionsPage.vue'),
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
