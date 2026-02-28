import Link from 'next/link';
import { ExternalLink, LineChart, TimerReset } from 'lucide-react';
import { getTimingCorrelations } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Stat } from '@/components/ui/Stat';
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/Table';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function TimingPage() {
  const correlations = await getTimingCorrelations();
  const flagged = correlations.filter((row) => row.flagged).length;
  const avgDays = Math.round(correlations.reduce((sum, row) => sum + row.daysBetween, 0) / Math.max(1, correlations.length));

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <div className="mb-8">
        <h1 className="text-3xl font-semibold text-white">Timing Correlations</h1>
        <p className="mt-1 max-w-3xl text-sm text-gray-400">
          Temporal analysis of official actions and related financial or influence events. Correlation indicates sequence, not proof of causation.
        </p>
      </div>

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <Stat label="Total rows" value={String(correlations.length)} />
        <Stat label="Flagged patterns" value={String(flagged)} trend="down" change="Requires review" />
        <Stat label="Average gap" value={`${avgDays} days`} />
        <Stat label="Shortest gap" value={`${Math.min(...correlations.map((row) => row.daysBetween))} days`} />
      </div>

      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <LineChart className="h-4 w-4 text-blue-300" /> Timeline visualization
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="flex h-56 items-center justify-center rounded-xl border border-dashed border-white/20 bg-gradient-to-br from-blue-500/10 to-transparent text-sm text-gray-400">
            Timeline chart placeholder (official event sequences)
          </div>
        </CardContent>
      </Card>

      <Card className="mt-6">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <TimerReset className="h-4 w-4 text-blue-300" /> Correlation table
          </CardTitle>
        </CardHeader>
        <CardContent className="p-0">
          <Table>
            <TableHead>
              <TableRow>
                <TableHeader>Official</TableHeader>
                <TableHeader>Event A</TableHeader>
                <TableHeader>Event B</TableHeader>
                <TableHeader>Days</TableHeader>
                <TableHeader>Type</TableHeader>
                <TableHeader>Sources</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {correlations.map((row) => (
                <TableRow key={row.id} flagged={row.flagged}>
                  <TableCell>
                    <Link href={`/entity/${row.officialId}`} className="text-white hover:text-blue-300">
                      {row.official}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <p className="text-sm text-gray-300">{row.eventA}</p>
                    <p className="text-xs text-gray-500">{row.eventADate}</p>
                  </TableCell>
                  <TableCell>
                    <p className="text-sm text-gray-300">{row.eventB}</p>
                    <p className="text-xs text-gray-500">{row.eventBDate}</p>
                  </TableCell>
                  <TableCell>
                    <span className="font-semibold text-white">{row.daysBetween}</span>
                  </TableCell>
                  <TableCell>
                    <Badge variant={row.flagged ? 'red' : 'blue'} className="capitalize">
                      {row.correlationType}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-col gap-1">
                      {row.sources.map((source) => (
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
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <SourceNote text="Source attribution: FEC filings, congressional records, LDA disclosures, and public court records." />
    </div>
  );
}
