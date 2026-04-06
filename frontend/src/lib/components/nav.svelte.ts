import Settings from '@lucide/svelte/icons/settings';
import House from '@lucide/svelte/icons/house';
import { Permission } from '$lib/permissions.svelte';
import Users from '@lucide/svelte/icons/users';
import User from '@lucide/svelte/icons/user';
import KeyRound from '@lucide/svelte/icons/key-round';
import DatabaseZap from '@lucide/svelte/icons/database-zap';
import type { NavGroup } from 'positron-components/components/nav/sidebar/types';

export const items: NavGroup[] = [
  {
    label: 'Cache',
    items: [
      { label: 'Overview', href: '/', icon: House },
      {
        label: 'Caches',
        href: '/caches',
        icon: DatabaseZap
      },
      {
        label: 'Access Tokens',
        href: '/tokens',
        icon: KeyRound
      }
    ]
  },
  {
    label: 'Administration',
    items: [
      {
        label: 'Users',
        href: '/users',
        icon: User,
        requiredPermission: Permission.USER_VIEW
      },
      {
        label: 'Groups',
        href: '/groups',
        icon: Users,
        requiredPermission: Permission.GROUP_VIEW
      },
      {
        label: 'Settings',
        href: '/settings',
        icon: Settings,
        requiredPermission: Permission.SETTINGS_VIEW
      }
    ]
  }
];

export const noSidebarPaths = [
  '/login',
  '/setup',
  '/password',
  '/password/forgot',
  '/password/reset'
];
