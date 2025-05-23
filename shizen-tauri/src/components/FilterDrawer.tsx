import { Fragment } from 'react';
import { Dialog, Transition } from '@headlessui/react';
import { XMarkIcon } from '@heroicons/react/24/outline';
import { useStore } from '../store';

interface FilterDrawerProps {
  isOpen: boolean;
  onClose: () => void;
}

export function FilterDrawer({ isOpen, onClose }: FilterDrawerProps) {
  // Get filter state and actions from store
  const filter = useStore(state => state.filter);
  const setFilter = useStore(state => state.setFilter);

  // Handle showing completed tasks
  const handleShowCompletedChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFilter({ showCompleted: e.target.checked });
  };

  // Handle showing blocked tasks
  const handleShowBlockedChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFilter({ showBlocked: e.target.checked });
  };

  // Handle search term change
  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setFilter({ searchTerm: e.target.value });
  };

  // Clear all filters
  const clearFilters = () => {
    setFilter({
      showCompleted: false,
      showBlocked: false,
      searchTerm: '',
      parentId: null,
    });
  };

  return (
    <Transition.Root show={isOpen} as={Fragment}>
      <Dialog as="div" className="relative z-10" onClose={onClose}>
        {/* Backdrop overlay */}
        <Transition.Child
          as={Fragment}
          enter="ease-in-out duration-300"
          enterFrom="opacity-0"
          enterTo="opacity-100"
          leave="ease-in-out duration-300"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <div className="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity" />
        </Transition.Child>

        {/* Drawer panel */}
        <div className="fixed inset-0 overflow-hidden">
          <div className="absolute inset-0 overflow-hidden">
            <div className="pointer-events-none fixed inset-y-0 right-0 flex max-w-full pl-10">
              <Transition.Child
                as={Fragment}
                enter="transform transition ease-in-out duration-300"
                enterFrom="translate-x-full"
                enterTo="translate-x-0"
                leave="transform transition ease-in-out duration-300"
                leaveFrom="translate-x-0"
                leaveTo="translate-x-full"
              >
                <Dialog.Panel className="pointer-events-auto relative w-screen max-w-md">
                  <div className="flex h-full flex-col overflow-y-auto bg-white dark:bg-gray-800 shadow-xl">
                    {/* Header */}
                    <div className="bg-primary-700 px-4 py-6 sm:px-6">
                      <div className="flex items-center justify-between">
                        <Dialog.Title className="text-base font-semibold leading-6 text-white">
                          Filter Tasks
                        </Dialog.Title>
                        <div className="ml-3 flex h-7 items-center">
                          <button
                            type="button"
                            className="rounded-md bg-primary-700 text-primary-200 hover:text-white focus:outline-none focus:ring-2 focus:ring-white"
                            onClick={onClose}
                          >
                            <span className="sr-only">Close panel</span>
                            <XMarkIcon className="h-6 w-6" aria-hidden="true" />
                          </button>
                        </div>
                      </div>
                    </div>

                    {/* Content */}
                    <div className="px-4 py-6 sm:px-6 space-y-6">
                      {/* Search */}
                      <div>
                        <label
                          htmlFor="search"
                          className="block text-sm font-medium text-gray-700 dark:text-gray-300"
                        >
                          Search
                        </label>
                        <div className="mt-1">
                          <input
                            type="text"
                            name="search"
                            id="search"
                            value={filter.searchTerm}
                            onChange={handleSearchChange}
                            className="block w-full rounded-md border-gray-300 dark:border-gray-600 shadow-sm focus:border-primary-500 focus:ring-primary-500 dark:bg-gray-700 dark:text-white sm:text-sm"
                            placeholder="Search by title or description"
                          />
                        </div>
                      </div>

                      {/* Show/Hide filters */}
                      <div className="space-y-4">
                        <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300">
                          Show/Hide
                        </h3>
                        <div className="flex items-center">
                          <input
                            id="show-completed"
                            name="show-completed"
                            type="checkbox"
                            checked={filter.showCompleted}
                            onChange={handleShowCompletedChange}
                            className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500 dark:border-gray-600 dark:bg-gray-700"
                          />
                          <label
                            htmlFor="show-completed"
                            className="ml-3 text-sm text-gray-700 dark:text-gray-300"
                          >
                            Show completed tasks
                          </label>
                        </div>
                        <div className="flex items-center">
                          <input
                            id="show-blocked"
                            name="show-blocked"
                            type="checkbox"
                            checked={filter.showBlocked}
                            onChange={handleShowBlockedChange}
                            className="h-4 w-4 rounded border-gray-300 text-primary-600 focus:ring-primary-500 dark:border-gray-600 dark:bg-gray-700"
                          />
                          <label
                            htmlFor="show-blocked"
                            className="ml-3 text-sm text-gray-700 dark:text-gray-300"
                          >
                            Show blocked tasks
                          </label>
                        </div>
                      </div>

                      {/* Clear filters */}
                      <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
                        <button
                          type="button"
                          onClick={clearFilters}
                          className="text-sm text-primary-600 hover:text-primary-700 dark:text-primary-400 dark:hover:text-primary-300"
                        >
                          Clear all filters
                        </button>
                      </div>

                      {/* Apply button */}
                      <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
                        <div className="flex justify-end">
                          <button
                            type="button"
                            className="rounded-md bg-primary-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-primary-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary-600"
                            onClick={onClose}
                          >
                            Apply Filters
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                </Dialog.Panel>
              </Transition.Child>
            </div>
          </div>
        </div>
      </Dialog>
    </Transition.Root>
  );
}