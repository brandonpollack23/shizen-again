import { useState } from 'react';
import { TaskList } from '../components/TaskList';
import { TaskDetail } from '../components/TaskDetail';
import { TaskToolbar } from '../components/TaskToolbar';
import { FilterDrawer } from '../components/FilterDrawer';

export function TasksPage() {
  const [filterOpen, setFilterOpen] = useState(false);
  
  return (
    <div className="h-full flex flex-col">
      <TaskToolbar onOpenFilter={() => setFilterOpen(true)} />
      
      <div className="flex-1 overflow-hidden flex flex-col md:flex-row">
        <div className="w-full md:w-1/2 lg:w-2/5 overflow-hidden flex flex-col">
          <TaskList />
        </div>
        
        <div className="w-full hidden md:block md:w-1/2 lg:w-3/5 border-l border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
          <TaskDetail />
        </div>
      </div>
      
      <FilterDrawer isOpen={filterOpen} onClose={() => setFilterOpen(false)} />
    </div>
  );
}