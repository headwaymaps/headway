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
    path: '/place/:placeId',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        name: 'place',
        path: '/place/:placeId',
        props: true,
        component: () => import('pages/PlacePage.vue'),
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
        component: () => import('src/pages/AlternatesPage.vue'),
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
        component: () => import('src/pages/AlternatesPage.vue'),
      },
    ],
  },
  {
    path: '/directions/:mode/:to/:from/:alternateIndex',
    component: () => import('layouts/MainLayout.vue'),
    children: [
      {
        path: '/directions/:mode/:to/:from/:alternateIndex',
        props: true,
        component: () => import('src/pages/StepsPage.vue'),
      },
    ],
  },
  // DEPRECATED ROUTE: will remove eventually
  {
    path: '/multimodal/:to/:from',
    redirect: (route) => {
      return {
        path: `/directions/transit/${encodeURIComponent(
          route.params.to.toString()
        )}/${encodeURIComponent(route.params.from.toString())}`,
      };
    },
  },

  // Always leave this as last one,
  // but you can also remove it
  {
    path: '/:catchAll(.*)*',
    component: () => import('pages/ErrorNotFound.vue'),
  },
];

export default routes;
