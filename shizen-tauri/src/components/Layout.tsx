import { useState } from 'react';
import { Link, Outlet, useLocation } from 'react-router-dom';
import { Bars3Icon, XMarkIcon, ListBulletIcon, Cog6ToothIcon } from '@heroicons/react/24/outline';
import { clsx } from 'clsx';

export function Layout() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const location = useLocation();

  const navigation = [
    { name: 'Tasks', to: '/', icon: ListBulletIcon },
    { name: 'Settings', to: '/settings', icon: Cog6ToothIcon },
  ];

  const isActive = (path: string) => {
    if (path === '/') {
      return location.pathname === '/';
    }
    return location.pathname.startsWith(path);
  };

  return (
    <div className="flex h-screen bg-gray-100 dark:bg-gray-900">
      {/* Mobile menu overlay */}
      {mobileMenuOpen && (
        <div className="fixed inset-0 z-40 lg:hidden">
          {/* Background overlay */}
          <div
            className="fixed inset-0 bg-gray-600 bg-opacity-75"
            onClick={() => setMobileMenuOpen(false)}
          />
          
          {/* Sidebar */}
          <div className="relative flex w-full max-w-xs flex-1 flex-col bg-white dark:bg-gray-800">
            <div className="absolute top-0 right-0 -mr-12 pt-2">
              <button
                type="button"
                className="ml-1 flex h-10 w-10 items-center justify-center rounded-full focus:outline-none focus:ring-2 focus:ring-inset focus:ring-white"
                onClick={() => setMobileMenuOpen(false)}
              >
                <span className="sr-only">Close sidebar</span>
                <XMarkIcon className="h-6 w-6 text-white" aria-hidden="true" />
              </button>
            </div>

            <div className="flex flex-1 flex-col overflow-y-auto pt-5 pb-4">
              <div className="flex flex-shrink-0 items-center px-4">
                <h1 className="text-xl font-bold text-primary-600">Shizen</h1>
              </div>
              <nav className="mt-5 flex-1 space-y-1 px-2">
                {navigation.map((item) => (
                  <Link
                    key={item.name}
                    to={item.to}
                    className={clsx(
                      isActive(item.to)
                        ? 'bg-gray-100 text-primary-600 dark:bg-gray-700 dark:text-primary-400'
                        : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-gray-100',
                      'group flex items-center px-2 py-2 text-base font-medium rounded-md'
                    )}
                    onClick={() => setMobileMenuOpen(false)}
                  >
                    <item.icon
                      className={clsx(
                        isActive(item.to)
                          ? 'text-primary-600 dark:text-primary-400'
                          : 'text-gray-400 group-hover:text-gray-500 dark:text-gray-400 dark:group-hover:text-gray-300',
                        'mr-4 h-6 w-6 flex-shrink-0'
                      )}
                      aria-hidden="true"
                    />
                    {item.name}
                  </Link>
                ))}
              </nav>
            </div>
          </div>
        </div>
      )}

      {/* Static sidebar for desktop */}
      <div className="hidden border-r border-gray-200 dark:border-gray-700 lg:fixed lg:inset-y-0 lg:z-50 lg:flex lg:w-64 lg:flex-col">
        <div className="flex flex-grow flex-col overflow-y-auto bg-white dark:bg-gray-800 pt-5 pb-4">
          <div className="flex flex-shrink-0 items-center px-6">
            <h1 className="text-2xl font-bold text-primary-600">Shizen</h1>
          </div>
          <nav className="mt-8 flex flex-1 flex-col px-4">
            <div className="space-y-1">
              {navigation.map((item) => (
                <Link
                  key={item.name}
                  to={item.to}
                  className={clsx(
                    isActive(item.to)
                      ? 'bg-gray-100 text-primary-600 dark:bg-gray-700 dark:text-primary-400'
                      : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900 dark:text-gray-300 dark:hover:bg-gray-700 dark:hover:text-white',
                    'group flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium'
                  )}
                >
                  <item.icon
                    className={clsx(
                      isActive(item.to)
                        ? 'text-primary-600 dark:text-primary-400'
                        : 'text-gray-500 group-hover:text-gray-500 dark:text-gray-400 dark:group-hover:text-gray-300',
                      'h-5 w-5 flex-shrink-0'
                    )}
                    aria-hidden="true"
                  />
                  {item.name}
                </Link>
              ))}
            </div>
          </nav>
        </div>
      </div>

      {/* Content area */}
      <div className="flex flex-1 flex-col lg:pl-64">
        <div className="sticky top-0 z-10 flex h-16 flex-shrink-0 bg-white dark:bg-gray-800 shadow-sm border-b border-gray-200 dark:border-gray-700 lg:hidden">
          <button
            type="button"
            className="px-4 text-gray-500 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-primary-500 lg:hidden"
            onClick={() => setMobileMenuOpen(true)}
          >
            <span className="sr-only">Open sidebar</span>
            <Bars3Icon className="h-6 w-6" aria-hidden="true" />
          </button>
          <div className="flex flex-1 items-center justify-center px-6">
            <h1 className="text-lg font-bold text-primary-600">Shizen</h1>
          </div>
        </div>

        <main className="flex-1 bg-gray-100 dark:bg-gray-900">
          <Outlet />
        </main>
      </div>
    </div>
  );
}