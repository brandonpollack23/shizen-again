import { useState, useEffect } from 'react';
import { useStore } from '../store';
import { PeerInfo } from '../types';
import { fetchPeers, addPeer, removePeer, startSyncServer, syncWithPeer } from '../api';
import { ArrowPathIcon, TrashIcon } from '@heroicons/react/24/outline';

export function SyncSettings() {
  const { peers, setPeers, setError } = useStore();
  const [address, setAddress] = useState('');
  const [localPort, setLocalPort] = useState('1701');
  const [serverPort, setServerPort] = useState('1701');
  const [serverRunning, setServerRunning] = useState(false);
  const [syncing, setSyncing] = useState<string | null>(null);
  const [syncResult, setSyncResult] = useState<{ peerId: string; result: string } | null>(null);

  useEffect(() => {
    loadPeers();
  }, []);

  const loadPeers = async () => {
    try {
      const peersData = await fetchPeers();
      setPeers(peersData);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load peers');
    }
  };

  const handleAddPeer = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await addPeer(address, localPort ? parseInt(localPort) : undefined);
      setAddress('');
      await loadPeers();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to add peer');
    }
  };

  const handleRemovePeer = async (peerId: string) => {
    try {
      await removePeer(peerId);
      await loadPeers();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to remove peer');
    }
  };

  const handleStartServer = async () => {
    try {
      await startSyncServer(parseInt(serverPort));
      setServerRunning(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start sync server');
    }
  };

  const handleSync = async (peerId: string) => {
    try {
      setSyncing(peerId);
      setSyncResult(null);
      const result = await syncWithPeer(peerId);
      setSyncResult({
        peerId,
        result: `Synced ${result.numChanges} changes. Clock: ${result.updatedPeerClock}`,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to sync with peer');
    } finally {
      setSyncing(null);
    }
  };

  return (
    <div className="p-4 space-y-6">
      <h2 className="text-lg font-medium text-gray-900 dark:text-white">Synchronization Settings</h2>

      <div className="bg-white dark:bg-gray-800 shadow rounded-lg p-4">
        <h3 className="text-md font-medium mb-2">Sync Server</h3>
        <div className="flex gap-2 mb-4">
          <div className="flex-1">
            <label
              htmlFor="serverPort"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Server Port
            </label>
            <input
              type="text"
              id="serverPort"
              value={serverPort}
              onChange={(e) => setServerPort(e.target.value)}
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
              disabled={serverRunning}
            />
          </div>
          <div className="flex items-end">
            <button
              onClick={handleStartServer}
              disabled={serverRunning}
              className="px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500 disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {serverRunning ? 'Server Running' : 'Start Server'}
            </button>
          </div>
        </div>

        <h3 className="text-md font-medium mt-4 mb-2">Add New Peer</h3>
        <form onSubmit={handleAddPeer} className="flex flex-col gap-4">
          <div>
            <label
              htmlFor="address"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Remote Address (e.g. 192.168.1.100:1701)
            </label>
            <input
              type="text"
              id="address"
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              placeholder="hostname:port"
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
              required
            />
          </div>

          <div>
            <label
              htmlFor="localPort"
              className="block text-sm font-medium text-gray-700 dark:text-gray-300"
            >
              Local Server Port (optional)
            </label>
            <input
              type="text"
              id="localPort"
              value={localPort}
              onChange={(e) => setLocalPort(e.target.value)}
              className="mt-1 block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm focus:outline-none focus:ring-primary-500 focus:border-primary-500 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            />
          </div>

          <div>
            <button
              type="submit"
              className="w-full px-4 py-2 border border-transparent rounded-md shadow-sm text-sm font-medium text-white bg-primary-600 hover:bg-primary-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-primary-500"
            >
              Add Peer
            </button>
          </div>
        </form>
      </div>

      <div className="bg-white dark:bg-gray-800 shadow rounded-lg p-4">
        <h3 className="text-md font-medium mb-4">Peer List</h3>
        {peers.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 text-sm">No peers configured yet.</p>
        ) : (
          <ul className="divide-y divide-gray-200 dark:divide-gray-700">
            {peers.map((peer: PeerInfo) => (
              <li key={peer.peer_id[0]} className="py-4">
                <div className="flex justify-between items-center">
                  <div>
                    <p className="text-sm font-medium text-gray-900 dark:text-white">
                      {peer.addr}
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-400">
                      ID: {peer.peer_id[0].substring(0, 8)}... • Clock: {peer.clock}
                    </p>
                    {syncResult?.peerId === peer.peer_id[0] && syncResult && (
                      <p className="text-xs text-green-600 dark:text-green-400 mt-1">
                        {syncResult.result}
                      </p>
                    )}
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={() => handleSync(peer.peer_id[0])}
                      disabled={syncing === peer.peer_id[0]}
                      className="p-2 rounded-full text-gray-600 hover:text-primary-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:text-primary-400 dark:hover:bg-gray-700"
                    >
                      {syncing === peer.peer_id[0] ? (
                        <div className="h-5 w-5 animate-spin rounded-full border-2 border-t-primary-500" />
                      ) : (
                        <ArrowPathIcon className="h-5 w-5" />
                      )}
                    </button>
                    <button
                      onClick={() => handleRemovePeer(peer.peer_id[0])}
                      className="p-2 rounded-full text-gray-600 hover:text-red-600 hover:bg-gray-100 dark:text-gray-300 dark:hover:text-red-400 dark:hover:bg-gray-700"
                    >
                      <TrashIcon className="h-5 w-5" />
                    </button>
                  </div>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}