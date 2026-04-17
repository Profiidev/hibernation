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
      let _ = invalidate((url) => url.pathname.startsWith('/api/settings'));
      break;
    }
    case UpdateType.User: {
      let _ = invalidate('/api/user/management');
      let _u = invalidate(`/api/user/management/${msg.uuid}`);
      let _g = invalidate('/api/group/users');
      // Same as current user
      if (msg.uuid === user) {
        let _i = invalidate('/api/user/info');
      }
      break;
    }
    case UpdateType.UserPermissions: {
      let _ = invalidate('/api/user/info');
      break;
    }
    case UpdateType.Group: {
      let _ = invalidate('/api/group');
      let _g = invalidate(`/api/group/${msg.uuid}`);
      let _u = invalidate('/api/user/management/groups');
      break;
    }
    case UpdateType.Token: {
      let _ = invalidate('/api/token');
      let _t = invalidate(`/api/token/${msg.uuid}`);
      break;
    }
    case UpdateType.Cache: {
      let _ = invalidate('/api/cache/management');
      let _c = invalidate(`/api/cache/management/${msg.uuid}`);
      let _g = invalidate('/api/group/caches');
      break;
    }
  }
};
