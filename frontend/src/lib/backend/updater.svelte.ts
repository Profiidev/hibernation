import { invalidate } from '$app/navigation';
import {
  connectWebsocket as connect,
  disconnectWebsocket as disconnect
} from 'positron-components/backend';

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

export const connectWebsocket = (user: string) => connect(user, handleMessage);
export const disconnectWebsocket = () => disconnect();

const handleMessage = (msg: UpdateMessage, user: string) => {
  switch (msg.type) {
    case UpdateType.Settings: {
      const _ = invalidate((url) => url.pathname.startsWith('/api/settings'));
      break;
    }
    case UpdateType.User: {
      const _ = invalidate('/api/user/management');
      const _u = invalidate(`/api/user/management/${msg.uuid}`);
      const _g = invalidate('/api/group/users');
      // Same as current user
      if (msg.uuid === user) {
        const _i = invalidate('/api/user/info');
      }
      break;
    }
    case UpdateType.UserPermissions: {
      const _ = invalidate('/api/user/info');
      break;
    }
    case UpdateType.Group: {
      const _ = invalidate('/api/group');
      const _g = invalidate(`/api/group/${msg.uuid}`);
      const _u = invalidate('/api/user/management/groups');
      break;
    }
    case UpdateType.Token: {
      const _ = invalidate('/api/token');
      const _t = invalidate(`/api/token/${msg.uuid}`);
      break;
    }
    case UpdateType.Cache: {
      const _ = invalidate('/api/cache/management');
      const _c = invalidate(`/api/cache/management/${msg.uuid}`);
      const _g = invalidate('/api/group/caches');
      break;
    }
    default: {
      break;
    }
  }
};
