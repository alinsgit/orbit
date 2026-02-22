import { Store } from '@tauri-apps/plugin-store';

const SETTINGS_KEY = 'orbit-settings';

export interface Settings {
  ports: {
    nginx: number;
    php_start: number;
    mariadb: number;
  };
  paths: {
    bin_dir: string;
    data_dir: string;
  };
  services: {
    auto_start: boolean;
    autostart_list: string[];
    default_php: string;
  };
}

const defaultSettings: Settings = {
  ports: {
    nginx: 80,
    php_start: 9000,
    mariadb: 3306,
  },
  paths: {
    bin_dir: '',
    data_dir: '',
  },
  services: {
    auto_start: false,
    autostart_list: [],
    default_php: '8.3',
  },
};

class SettingsStore {
  private store: Store | null = null;

  async init() {
    this.store = await Store.load(SETTINGS_KEY);
  }

  async getSettings(): Promise<Settings> {
    if (!this.store) {
      await this.init();
    }

    const saved = await this.store?.get<Settings>('settings');
    return {
      ...defaultSettings,
      ...saved,
      services: { ...defaultSettings.services, ...saved?.services },
    };
  }

  async saveSettings(settings: Partial<Settings>): Promise<void> {
    if (!this.store) {
      await this.init();
    }

    const current = await this.getSettings();
    const merged = { ...current, ...settings };

    await this.store?.set('settings', merged);
  }

  async resetSettings(): Promise<void> {
    if (!this.store) {
      await this.init();
    }

    await this.store?.set('settings', defaultSettings);
  }
}

export const settingsStore = new SettingsStore();
