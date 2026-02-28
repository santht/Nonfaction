import { notFound } from 'next/navigation';
import Link from 'next/link';
import {
  ArrowLeft,
  AlertTriangle,
  ExternalLink,
  Calendar,
  Link2,
} from 'lucide-react';
import {
  getEntity,
  getEntityConnections,
  type Entity,
} from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent } from '@/components/ui/Card';
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/Tabs';

const TYPE_BADGE: Record<
  Entity['type'],
  { variant: 'blue' | 'default' | 'yellow' | 'green' | 'red'; label: string }
> = {
  politician: { variant: 'blue', label: 'Politician' },
  corporation: { variant: 'default', label: 'Corporation' },
  lobbyist: { variant: 'yellow', label: 'Lobbyist' },
  nonprofit: { variant: 'green', label: 'Nonprofit' },
  donor: { variant: 'default', label: 'Donor' },
};

const MOCK_TIMELINE = [
  {
    date: '2024-02-14',
    event: 'Received $450,000 from Aerospace Defense PAC',
    type: 'financial',
    source: { title: 'FEC Filing Q3 2024', url: 'https://www.fec.gov/data/receipts/?committee_id=C00123456' },
  },
  {
    date: '2024-03-22',
    event: 'Voted YES on SB-2024-0042 ($8.2B Defense Appropriations)',
    type: 'vote',
    source: { title: 'Senate Vote Record', url: 'https://www.congress.gov/bill/118th-congress/senate-bill/42' },
  },
  {
    date: '2024-06-05',
    event: 'Attended Aerospace Consortium annual gala',
    type: 'meeting',
    source: { title: 'Reuters Investigation', url: 'https://www.reuters.com/investigates/defense-pac-senators-2024/' },
  },
  {
    date: '2024-06-19',
    event: 'Blocked floor vote on defense contractor oversight amendment',
    type: 'vote',
    source: { title: 'Congress.gov Roll Call', url: 'https://www.congress.gov/bill/118th-congress/senate-bill/42' },
  },
  {
    date: '2024-08-01',
    event: 'Used official travel budget for 14 trips to lobbying firm city',
    type: 'conduct',
    source: { title: 'Reuters Investigation', url: 'https://www.reuters.com/investigates/defense-pac-senators-2024/' },
  },
];

const TYPE_COLOR: Record<string, string> = {
  financial: 'text-yellow-400',
  vote: 'text-blue-400',
  meeting: 'text-purple-400',
  conduct: 'text-red-400',
};

export default async function EntityPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const [entity, connections] = await Promise.all([
    getEntity(id),
    getEntityConnections(id),
  ]);

  if (!entity) notFound();

  const badge = TYPE_BADGE[entity.type];

  return (
    <div className="max-w-5xl mx-auto px-4 sm:px-6 py-10">
      {/* Back */}
      <Link
        href="/search"
        className="inline-flex items-center gap-1.5 text-sm text-gray-500 hover:text-white transition-colors mb-8"
      >
        <ArrowLeft className="w-3.5 h-3.5" />
        Back to search
      </Link>

      {/* Hero card */}
      <Card className="p-8 mb-8">
        <CardContent className="p-0">
          <div className="flex flex-col sm:flex-row sm:items-start sm:justify-between gap-6">
            <div>
              <div className="flex flex-wrap items-center gap-2 mb-3">
                <Badge variant={badge.variant}>{badge.label}</Badge>
                {entity.flagged && (
                  <Badge variant="red">
                    <AlertTriangle className="w-2.5 h-2.5" />
                    Flagged
                  </Badge>
                )}
                {entity.party && (
                  <Badge variant="outline">{entity.party}</Badge>
                )}
                {entity.state && (
                  <Badge variant="outline">{entity.state}</Badge>
                )}
              </div>
              <h1 className="text-3xl font-bold text-white mb-2">
                {entity.name}
              </h1>
              {entity.role && (
                <p className="text-gray-400">{entity.role}</p>
              )}
              <p className="text-xs text-gray-600 mt-3">
                Last updated: {entity.lastUpdated}
              </p>
            </div>

            <div className="flex gap-8 shrink-0">
              <div className="text-center">
                <div className="text-3xl font-bold text-white">
                  {entity.connectionCount}
                </div>
                <div className="text-xs text-gray-500 mt-0.5">Connections</div>
              </div>
              <div className="text-center">
                <div className="text-3xl font-bold text-blue-400">
                  {entity.sourceCount}
                </div>
                <div className="text-xs text-gray-500 mt-0.5">Sources</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Tabs */}
      <Tabs defaultValue="overview">
        <TabsList>
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="connections">Connections</TabsTrigger>
          <TabsTrigger value="timeline">Timeline</TabsTrigger>
          <TabsTrigger value="sources">Sources</TabsTrigger>
          <TabsTrigger value="timing">Timing</TabsTrigger>
        </TabsList>

        {/* Overview */}
        <TabsContent value="overview">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <Card className="p-6">
              <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
                Profile
              </h3>
              <dl className="space-y-3">
                {[
                  { label: 'Type', value: entity.type },
                  { label: 'Role', value: entity.role ?? '—' },
                  { label: 'Party', value: entity.party ?? '—' },
                  { label: 'State', value: entity.state ?? '—' },
                  { label: 'Status', value: entity.flagged ? 'Flagged' : 'Monitored' },
                ].map(({ label, value }) => (
                  <div key={label} className="flex justify-between">
                    <dt className="text-sm text-gray-500">{label}</dt>
                    <dd className={`text-sm font-medium ${value === 'Flagged' ? 'text-red-400' : 'text-white'}`}>
                      {value}
                    </dd>
                  </div>
                ))}
              </dl>
            </Card>

            <Card className="p-6">
              <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
                Activity Summary
              </h3>
              <dl className="space-y-3">
                {[
                  { label: 'Total Connections', value: entity.connectionCount.toString() },
                  { label: 'Verified Sources', value: entity.sourceCount.toString() },
                  { label: 'Last Updated', value: entity.lastUpdated },
                  { label: 'Flagged Connections', value: entity.flagged ? 'Yes' : 'None' },
                ].map(({ label, value }) => (
                  <div key={label} className="flex justify-between">
                    <dt className="text-sm text-gray-500">{label}</dt>
                    <dd className="text-sm font-medium text-white">{value}</dd>
                  </div>
                ))}
              </dl>
            </Card>
          </div>
        </TabsContent>

        {/* Connections */}
        <TabsContent value="connections">
          <div className="space-y-4">
            {connections.length === 0 ? (
              <p className="text-gray-500 text-sm">No connections on file.</p>
            ) : (
              connections.map((conn) => (
                <Card key={conn.id} className="p-5">
                  <CardContent className="p-0">
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1">
                        <div className="flex items-center gap-2 mb-1.5">
                          <Link2 className="w-4 h-4 text-blue-400" />
                          <span className="text-sm font-semibold text-white capitalize">
                            {conn.type} Connection
                          </span>
                          {conn.amount && (
                            <Badge variant="yellow">
                              ${conn.amount.toLocaleString()}
                            </Badge>
                          )}
                        </div>
                        <p className="text-sm text-gray-400 mb-1.5">
                          {conn.description}
                        </p>
                        <div className="flex items-center gap-1.5 text-xs text-gray-600">
                          <Calendar className="w-3 h-3" />
                          {conn.date}
                        </div>
                      </div>
                      <Link
                        href={`/entity/${conn.toEntity.id}`}
                        className="text-sm text-blue-400 hover:text-blue-300 shrink-0 flex items-center gap-1"
                      >
                        {conn.toEntity.name}
                        <ExternalLink className="w-3 h-3" />
                      </Link>
                    </div>
                    <div className="mt-3 pt-3 border-t border-white/6">
                      <p className="text-xs text-gray-600 mb-1.5">Sources:</p>
                      <div className="flex flex-wrap gap-2">
                        {conn.sources.map((src) => (
                          <a
                            key={src.id}
                            href={src.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300 transition-colors"
                          >
                            <ExternalLink className="w-2.5 h-2.5" />
                            {src.publisher}
                          </a>
                        ))}
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))
            )}
          </div>
        </TabsContent>

        {/* Timeline */}
        <TabsContent value="timeline">
          <div className="relative pl-6">
            <div className="absolute left-0 top-0 bottom-0 w-px bg-white/10" />
            <div className="space-y-6">
              {MOCK_TIMELINE.map((item, i) => (
                <div key={i} className="relative">
                  <div
                    className={`absolute -left-[1.5rem] top-1 w-2 h-2 rounded-full border-2 border-[#0A0F1C] ${
                      item.type === 'financial'
                        ? 'bg-yellow-400'
                        : item.type === 'vote'
                        ? 'bg-blue-400'
                        : item.type === 'conduct'
                        ? 'bg-red-400'
                        : 'bg-purple-400'
                    }`}
                  />
                  <div className="ml-2">
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`text-xs font-medium capitalize ${TYPE_COLOR[item.type] ?? 'text-gray-400'}`}>
                        {item.type}
                      </span>
                      <span className="text-xs text-gray-600">{item.date}</span>
                    </div>
                    <p className="text-sm text-white mb-1.5">{item.event}</p>
                    <a
                      href={item.source.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300"
                    >
                      <ExternalLink className="w-2.5 h-2.5" />
                      {item.source.title}
                    </a>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </TabsContent>

        {/* Sources */}
        <TabsContent value="sources">
          <div className="space-y-3">
            {connections
              .flatMap((c) => c.sources)
              .map((src) => (
                <Card key={src.id} className="p-4">
                  <CardContent className="p-0">
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <div className="flex items-center gap-2 mb-1">
                          <Badge variant="outline" className="capitalize">
                            {src.type}
                          </Badge>
                          <span className="text-xs text-gray-600">
                            {src.publishedDate}
                          </span>
                        </div>
                        <p className="text-sm font-medium text-white mb-0.5">
                          {src.title}
                        </p>
                        <p className="text-xs text-gray-500">{src.publisher}</p>
                      </div>
                      <a
                        href={src.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="shrink-0 flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300 px-3 py-1.5 rounded-lg border border-blue-500/20 hover:border-blue-500/40 transition-all"
                      >
                        View Source
                        <ExternalLink className="w-3 h-3" />
                      </a>
                    </div>
                  </CardContent>
                </Card>
              ))}
          </div>
        </TabsContent>

        {/* Timing */}
        <TabsContent value="timing">
          <Card className="p-6">
            <p className="text-sm text-gray-400 mb-4">
              Timing correlations show days between financial events and official
              actions. All events are sourced from public records.
            </p>
            <div className="space-y-4">
              {[
                { eventA: 'PAC donation received', eventB: 'Voted YES on defense bill', days: 37, flagged: true },
                { eventA: 'Attended industry gala', eventB: 'Blocked oversight amendment', days: 14, flagged: true },
              ].map((t, i) => (
                <div
                  key={i}
                  className={`p-4 rounded-xl border ${
                    t.flagged
                      ? 'border-red-500/20 bg-red-500/5'
                      : 'border-white/8 bg-white/3'
                  }`}
                >
                  <div className="flex items-center justify-between gap-4">
                    <div className="text-sm text-gray-300">
                      <span className="text-white font-medium">{t.eventA}</span>
                      <span className="text-gray-600 mx-2">→</span>
                      <span className="text-white font-medium">{t.eventB}</span>
                    </div>
                    <div className="text-right shrink-0">
                      <div className={`text-xl font-bold ${t.flagged ? 'text-red-400' : 'text-white'}`}>
                        {t.days}d
                      </div>
                      {t.flagged && (
                        <Badge variant="red" className="mt-1">
                          <AlertTriangle className="w-2.5 h-2.5" />
                          Flagged
                        </Badge>
                      )}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </Card>
        </TabsContent>
      </Tabs>
    </div>
  );
}
