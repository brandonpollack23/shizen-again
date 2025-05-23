import { createBrowserRouter } from 'react-router-dom';
import { Layout } from './components/Layout';
import { TasksPage } from './pages/TasksPage';
import { SettingsPage } from './pages/SettingsPage';

export const router = createBrowserRouter([
  {
    path: '/',
    element: <Layout />,
    children: [
      {
        index: true,
        element: <TasksPage />,
      },
      {
        path: 'settings',
        element: <SettingsPage />,
      },
    ],
  },
]);