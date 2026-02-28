'use client';

import { useEffect, useState } from 'react';
import { Bell, Plus, Trash2 } from 'lucide-react';
import { getAlerts, getWatchlistEntries, type Alert, type WatchlistEntry } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { EmptyState } from '@/components/ui/EmptyState';
import { Select } from '@/components/ui/Select';
import { Skeleton } from '@/components/ui/Skeleton';
import { SourceNote } from '@/components/ui/SourceNote';

export default function WatchlistPage() {
  const [entries, setEntries] = useState<WatchlistEntry[]>([]);
  const [alerts, setAlerts] = useState<Alert[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    Promise.all([getWatchlistEntries(), getAlerts()]).then(([watchlist, latestAlerts]) => {
      setEntries(watchlist);
      setAlerts(latestAlerts);
      setLoading(false);
    });
  }, []);

  if (loading) {
    return (
      <div className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
        <Skeleton className="h-24" />
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Watchlist Dashboard</h1>
      <p className="mt-1 text-sm text-gray-400">Subscribed entities, alert preferences, and recent watchlist signals.</p>

      {entries.length === 0 ? (
        <div className="mt-6">
          <EmptyState icon={Bell} title="No watchlist entries" description="Add entities from profile pages to receive alert updates." ctaLabel="Browse entities" ctaHref="/search" />
        </div>
      ) : (
        <div className="mt-6 grid gap-4 lg:grid-cols-[1fr_0.9fr]">
          <Card>
            <CardHeader>
              <div className="flex items-center justify-between">
                <CardTitle className="text-base">Subscribed entities</CardTitle>
                <Button size="sm"><Plus className="h-3.5 w-3.5" /> Add</Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              {entries.map((entry) => (
                <div key={entry.id} className="rounded-xl border border-white/10 bg-white/3 p-3">
                  <div className="flex items-center justify-between gap-2">
                    <div>
                      <p className="text-sm font-medium text-white">{entry.entityName}</p>
                      <p className="text-xs text-gray-500">Added {entry.createdAt}</p>
                    </div>
                    <Badge variant="blue" className="capitalize">{entry.entityType}</Badge>
                  </div>
                  <div className="mt-2 flex items-center gap-2">
                    <Select defaultValue={entry.alertPreference}>
                      <option value="immediate" className="bg-[#0a0f1c]">Immediate</option>
                      <option value="daily" className="bg-[#0a0f1c]">Daily</option>
                      <option value="weekly" className="bg-[#0a0f1c]">Weekly</option>
                    </Select>
                    <Button variant="ghost" size="sm"><Trash2 className="h-3.5 w-3.5" /></Button>
                  </div>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">Recent alerts feed</CardTitle></CardHeader>
            <CardContent className="space-y-3">
              {alerts.map((alert) => (
                <div key={alert.id} className="rounded-xl border border-white/10 bg-white/3 p-3">
                  <div className="flex items-center justify-between gap-2">
                    <p className="text-sm font-medium text-white">{alert.title}</p>
                    <Badge variant={alert.severity === 'high' ? 'red' : alert.severity === 'medium' ? 'yellow' : 'green'}>
                      {alert.severity}
                    </Badge>
                  </div>
                  <p className="mt-1 text-xs text-gray-400">{alert.summary}</p>
                  <p className="mt-1 text-xs text-gray-500">{alert.createdAt}</p>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>
      )}

      <SourceNote text="Source attribution: alerts are derived from source-linked updates across tracked public records." />
    </div>
  );
}
