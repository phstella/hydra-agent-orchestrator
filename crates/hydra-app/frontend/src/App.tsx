import { useState, useCallback, useEffect } from 'react';
import { PreflightDashboard } from './components/PreflightDashboard';
import { ExperimentalAdapterModal } from './components/ExperimentalAdapterModal';
import { Tabs } from './components/design-system';
import { listAdapters } from './ipc';
import type { AdapterInfo } from './types';
import { isExperimental } from './types';

const NAV_TABS = [
  { id: 'preflight', label: 'Preflight' },
  { id: 'race', label: 'Race', badge: 'soon' },
  { id: 'results', label: 'Results', badge: 'soon' },
];

export default function App() {
  const [activeTab, setActiveTab] = useState('preflight');
  const [experimentalModal, setExperimentalModal] = useState<{
    open: boolean;
    adapter: AdapterInfo | null;
  }>({ open: false, adapter: null });
  const [adapters, setAdapters] = useState<AdapterInfo[]>([]);

  useEffect(() => {
    listAdapters().then(setAdapters).catch(() => {});
  }, []);

  const handleAdapterSelect = useCallback(
    (adapterKey: string) => {
      const adapter = adapters.find((a) => a.key === adapterKey);
      if (!adapter) return;

      if (isExperimental(adapter)) {
        setExperimentalModal({ open: true, adapter });
      }
    },
    [adapters],
  );

  const handleExperimentalConfirm = useCallback(() => {
    setExperimentalModal({ open: false, adapter: null });
  }, []);

  const handleExperimentalClose = useCallback(() => {
    setExperimentalModal({ open: false, adapter: null });
  }, []);

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      {/* Top nav bar */}
      <header
        style={{
          backgroundColor: 'var(--color-bg-900)',
          borderBottom: '1px solid var(--color-border-700)',
          padding: '0 var(--space-6)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          height: 52,
          flexShrink: 0,
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
          <span
            style={{
              fontSize: 'var(--text-lg)',
              fontWeight: 'var(--weight-bold)' as unknown as number,
              color: 'var(--color-green-400)',
              fontFamily: 'var(--font-mono)',
            }}
          >
            ⟁ Hydra
          </span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 'var(--space-4)' }}>
          <span style={{ fontSize: 'var(--text-xs)', color: 'var(--color-text-muted)' }}>
            v0.1.0-alpha
          </span>
          {/* Adapter quick-select for testing experimental modal */}
          {adapters.filter(isExperimental).map((a) => (
            <button
              key={a.key}
              onClick={() => handleAdapterSelect(a.key)}
              style={{
                background: 'transparent',
                border: '1px solid var(--color-warning-500)',
                borderRadius: 'var(--radius-md)',
                padding: '2px var(--space-2)',
                fontSize: 'var(--text-xs)',
                color: 'var(--color-warning-400)',
                cursor: 'pointer',
                fontFamily: 'var(--font-family)',
              }}
            >
              + {a.key}
            </button>
          ))}
        </div>
      </header>

      {/* Tab navigation */}
      <Tabs tabs={NAV_TABS} activeTab={activeTab} onTabChange={setActiveTab}>
        <main style={{ flex: 1 }}>
          {activeTab === 'preflight' && <PreflightDashboard />}
          {activeTab === 'race' && (
            <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
              Race view coming in P3-UI-03
            </div>
          )}
          {activeTab === 'results' && (
            <div style={{ padding: 'var(--space-8)', textAlign: 'center', color: 'var(--color-text-muted)' }}>
              Results view coming in P3-UI-04
            </div>
          )}
        </main>
      </Tabs>

      {/* Experimental adapter warning modal */}
      <ExperimentalAdapterModal
        open={experimentalModal.open}
        onClose={handleExperimentalClose}
        onConfirm={handleExperimentalConfirm}
        adapter={experimentalModal.adapter}
      />
    </div>
  );
}
