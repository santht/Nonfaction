'use client';

import { useEffect, useState } from 'react';
import Link from 'next/link';
import { ArrowLeft, Download, ExternalLink, Network, TrendingUp, Vote } from 'lucide-react';
import { getEntity, getEntityConnections, getTimingCorrelations, getRelatedEntities, type Entity, type Connection, type TimingCorrelation } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Breadcrumb } from '@/components/ui/Breadcrumb';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';
import { Skeleton } from '@/components/ui/Skeleton';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/Table';

export default function EntityClient({ id }: { id: string }) {
  const [entity, setEntity] = useState<Entity | null>(null);
  const [connections, setConnections] = useState<Connection[]>([]);
  const [entityTiming, setEntityTiming] = useState<TimingCorrelation[]>([]);
  const [related, setRelated] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function load() {
      setLoading(true);
      const e = await getEntity(id);
      if (!e) { setLoading(false); return; }
      setEntity(e);

      const [conn, timing, rel] = await Promise.all([
        getEntityConnections(e.id),
        getTimingCorrelations({ flaggedOnly: true }),
        getRelatedEntities(['e2', 'e4', 'e5', 'e7']),
      ]);
      setConnections(conn);
      setEntityTiming(timing.filter((item) => item.official.toLowerCase().includes(e.name.split(' ')[1]?.toLowerCase() ?? '')));
      setRelated(rel);
      setLoading(false);
    }
    load();
  }, [id]);

  if (loading) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6 space-y-4">
        <Skeleton className="h-8 w-64" />
        <Skeleton className="h-48 w-full" />
        <Skeleton className="h-48 w-full" />
      </div>
    );
  }

  if (!entity) {
    return (
      <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
        <p className="text-gray-400">Entity not found.</p>
        <Link href="/search" className="mt-2 inline-block text-sm text-blue-300 hover:text-blue-200">Back to search</Link>
      </div>
    );
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <Breadcrumb
        items={[
          { label: 'Home', href: '/' },
          { label: 'Search', href: '/search' },
          { label: entity.name },
        ]}
      />

      <Link href="/search" className="mb-4 inline-flex items-center gap-1 text-sm text-gray-300 hover:text-white">
        <ArrowLeft className="h-4 w-4" /> Back to search
      </Link>

      <div className="grid gap-6 lg:grid-cols-[1.3fr_0.7fr]">
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex flex-wrap items-center gap-2">
                <CardTitle>{entity.name}</CardTitle>
                <Badge variant="blue" className="capitalize">{entity.type}</Badge>
                {entity.flagged ? <Badge variant="red">Flagged</Badge> : null}
              </div>
              <CardDescription>
                {entity.role} {entity.state ? `· ${entity.state}` : ''} {entity.party ? `· ${entity.party}` : ''}
              </CardDescription>
            </CardHeader>
            <CardContent className="grid gap-4 sm:grid-cols-3">
              <div className="rounded-xl border border-white/10 bg-white/4 p-4">
                <p className="text-xs text-gray-500">Connections</p>
                <p className="text-2xl font-semibold text-white">{entity.connectionCount}</p>
              </div>
              <div className="rounded-xl border border-white/10 bg-white/4 p-4">
                <p className="text-xs text-gray-500">Sources</p>
                <p className="text-2xl font-semibold text-white">{entity.sourceCount}</p>
              </div>
              <div className="rounded-xl border border-white/10 bg-white/4 p-4">
                <p className="text-xs text-gray-500">Updated</p>
                <p className="text-lg font-semibold text-white">{entity.lastUpdated}</p>
              </div>
            </CardContent>
          </Card>

          <div className="grid gap-6 md:grid-cols-2">
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <TrendingUp className="h-4 w-4 text-blue-300" /> Financial summary chart
                </CardTitle>
                <CardDescription>Placeholder for donations and spending trend visualization.</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex h-48 items-center justify-center rounded-xl border border-dashed border-white/20 bg-gradient-to-b from-blue-500/10 to-transparent text-sm text-gray-400">
                  Chart integration placeholder
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2 text-base">
                  <Network className="h-4 w-4 text-blue-300" /> Network graph
                </CardTitle>
                <CardDescription>Placeholder for node-link graph view with relationship filtering.</CardDescription>
              </CardHeader>
              <CardContent>
                <div className="flex h-48 items-center justify-center rounded-xl border border-dashed border-white/20 bg-white/4 text-sm text-gray-400">
                  Graph canvas placeholder
                </div>
              </CardContent>
            </Card>
          </div>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Connections</CardTitle>
            </CardHeader>
            <CardContent className="overflow-x-auto p-0">
              <Table>
                <TableHead>
                  <TableRow>
                    <TableHeader>Type</TableHeader>
                    <TableHeader>Description</TableHeader>
                    <TableHeader>Amount</TableHeader>
                    <TableHeader>Date</TableHeader>
                    <TableHeader>Sources</TableHeader>
                  </TableRow>
                </TableHead>
                <TableBody>
                  {connections.map((connection) => (
                    <TableRow key={connection.id}>
                      <TableCell className="capitalize">{connection.type}</TableCell>
                      <TableCell>{connection.description}</TableCell>
                      <TableCell>{connection.amount ? `$${connection.amount.toLocaleString()}` : '—'}</TableCell>
                      <TableCell>{connection.date}</TableCell>
                      <TableCell>
                        {connection.sources.map((source) => (
                          <a
                            key={source.id}
                            href={source.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-1 text-xs text-blue-300 hover:text-blue-200"
                          >
                            <ExternalLink className="h-3 w-3" />
                            {source.publisher}
                          </a>
                        ))}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-base">
                <Vote className="h-4 w-4 text-blue-300" /> Voting record highlights
              </CardTitle>
            </CardHeader>
            <CardContent>
              <div className="space-y-3">
                {entityTiming.length ? (
                  entityTiming.map((row) => (
                    <div key={row.id} className="rounded-xl border border-white/10 bg-white/3 p-4">
                      <p className="text-sm text-white">{row.eventB}</p>
                      <p className="mt-1 text-xs text-gray-400">{row.eventBDate} · {row.daysBetween} days from preceding event</p>
                    </div>
                  ))
                ) : (
                  <p className="text-sm text-gray-400">No voting highlights in current scope.</p>
                )}
              </div>
            </CardContent>
          </Card>

          <SourceNote text="Data attribution: FEC disclosures, Senate LDA, Congress roll calls, and CourtListener dockets." />
        </div>

        <aside className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Story package</CardTitle>
              <CardDescription>Export source-linked dossier for reporting workflows.</CardDescription>
            </CardHeader>
            <CardContent>
              <Button className="w-full">
                <Download className="h-4 w-4" /> Download package (.zip)
              </Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle className="text-base">Related entities</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {related.map((item) => (
                <Link
                  key={item.id}
                  href={`/entity/${item.id}`}
                  className="block rounded-xl border border-white/10 bg-white/4 px-3 py-2 text-sm text-gray-200 hover:border-white/20"
                >
                  <div className="flex items-center justify-between gap-2">
                    <span>{item.name}</span>
                    {item.flagged ? <Badge variant="red">Flagged</Badge> : <Badge variant="outline">Tracked</Badge>}
                  </div>
                  <p className="mt-1 text-xs text-gray-500">{item.connectionCount} connections</p>
                </Link>
              ))}
            </CardContent>
          </Card>
        </aside>
      </div>
    </div>
  );
}
