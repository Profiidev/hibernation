import { browser } from '$app/environment';
import { invalidate } from '$app/navigation';
import { sleep } from 'positron-components/util/interval.svelte';

export enum UpdateType {
  Settings = 'Settings',
  User = 'User',
  UserPermissions = 'UserPermissions',
  Group = 'Group',
  Token = 'Token',
  Cache = 'Cache'
}

export type UpdateMessage =
  | {
      type:
        | UpdateType.User
        | UpdateType.Group
        | UpdateType.Token
        | UpdateType.Cache;
      uuid: string;
    }
  | {
      type: UpdateType.Settings | UpdateType.UserPermissions;
    };

let updater: WebSocket | undefined | false = $state(browser && undefined);
let interval: number;
let disconnect = false;

export const connectWebsocket = (user: string) => {
  if (updater === false || updater) return;
  createWebsocket(user);
};

const createWebsocket = (user: string) => {
  updater = new WebSocket('/api/ws/updater');

  updater.onmessage = (event) => {
    const msg: UpdateMessage = JSON.parse(event.data);
    handleMessage(msg, user);
  };

  updater.onclose = async () => {
    clearInterval(interval);
    if (disconnect) return;
    await sleep(1000);
    createWebsocket(user);
  };

  interval = setInterval(() => {
    if (
      !updater ||
      updater.readyState === updater.CLOSING ||
      updater.readyState === updater.CLOSED
    ) {
      clearInterval(interval);
      return;
    }

    updater.send('heartbeat');
  }, 10000) as unknown as number;
};

export const disconnectWebsocket = () => {
  if (updater) {
    disconnect = true;
    updater.close();
    updater = undefined;
  }
};

const handleMessage = (msg: UpdateMessage, user: string) => {
  switch (msg.type) {
    case UpdateType.Settings: {
      invalidate((url) => url.pathname.startsWith('/api/settings'));
      break;
    }
    case UpdateType.User: {
      invalidate('/api/user/management');
      invalidate(`/api/user/management/${msg.uuid}`);
      invalidate('/api/group/users');
      // Same as current user
      if (msg.uuid === user) {
        invalidate('/api/user/info');
      }
      break;
    }
    case UpdateType.UserPermissions: {
      invalidate('/api/user/info');
      break;
    }
    case UpdateType.Group: {
      invalidate('/api/group');
      invalidate(`/api/group/${msg.uuid}`);
      invalidate('/api/user/management/groups');
      break;
    }
    case UpdateType.Token: {
      invalidate('/api/token');
      invalidate(`/api/token/${msg.uuid}`);
      break;
    }
    case UpdateType.Cache: {
      invalidate('/api/cache/management');
      invalidate(`/api/cache/management/${msg.uuid}`);
      invalidate('/api/group/caches');
      break;
    }
  }
};
