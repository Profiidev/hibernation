import Settings from '@lucide/svelte/icons/settings';
import House from '@lucide/svelte/icons/house';
import { Permission } from '$lib/permissions.svelte';
import type { Component } from 'svelte';
import Users from '@lucide/svelte/icons/users';
import User from '@lucide/svelte/icons/user';
import KeyRound from '@lucide/svelte/icons/key-round';
import DatabaseZap from '@lucide/svelte/icons/database-zap';

export interface NavGroup {
  label: string;
  items: NavItem[];
}

export interface NavItem {
  label: string;
  href: string;
  icon?: Component;
  requiredPermission?: Permission;
}

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
