import { SyncSettings } from '../components/SyncSettings';

export function SettingsPage() {
  return (
    <div className="bg-gray-100 dark:bg-gray-900 min-h-full">
      <div className="max-w-4xl mx-auto py-6 px-4 sm:px-6 lg:px-8">
        <h1 className="text-2xl font-semibold text-gray-900 dark:text-white mb-6">Settings</h1>
        
        <div className="bg-white dark:bg-gray-800 shadow rounded-lg">
          <SyncSettings />
        </div>
      </div>
    </div>
  );
}