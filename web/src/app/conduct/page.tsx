import Link from 'next/link';
import { ExternalLink, AlertTriangle } from 'lucide-react';
import { getConductRows } from '@/lib/api';
import {
  Table,
  TableHead,
  TableBody,
  TableRow,
  TableHeader,
  TableCell,
} from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';

export default async function ConductPage() {
  const rows = await getConductRows();

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 py-10">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-white mb-2">
          Conduct Comparison
        </h1>
        <p className="text-gray-400 max-w-2xl">
          Official actions documented alongside their equivalent in private
          conduct — and the consequences applied to each. Every row sourced from
          public records.
        </p>
      </div>

      {/* Info banner */}
      <div className="flex items-start gap-3 mb-6 p-4 rounded-xl bg-blue-500/5 border border-blue-500/10">
        <AlertTriangle className="w-4 h-4 text-blue-400 shrink-0 mt-0.5" />
        <p className="text-sm text-blue-300">
          This table presents documented official actions and analogous private
          conduct for contextual comparison only. No editorial judgment is
          implied. All sources linked.
        </p>
      </div>

      {/* Table */}
      <div className="rounded-xl border border-white/8 overflow-hidden">
        <Table>
          <TableHead>
            <TableRow>
              <TableHeader>Official Action</TableHeader>
              <TableHeader>Official</TableHeader>
              <TableHeader>Date</TableHeader>
              <TableHeader>Source</TableHeader>
              <TableHeader>Equivalent Private Conduct</TableHeader>
              <TableHeader>Consequence</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {rows.map((row) => (
              <TableRow key={row.id} flagged>
                <TableCell>
                  <p className="text-sm text-gray-200 max-w-xs leading-relaxed">
                    {row.officialAction}
                  </p>
                </TableCell>
                <TableCell>
                  <Link
                    href={`/entity/${row.officialId}`}
                    className="text-white font-medium hover:text-blue-400 transition-colors whitespace-nowrap text-sm"
                  >
                    {row.official}
                  </Link>
                  <p className="text-xs text-gray-600 mt-0.5">{row.date}</p>
                </TableCell>
                <TableCell>
                  <span className="text-xs text-gray-500">{row.date}</span>
                </TableCell>
                <TableCell>
                  <a
                    href={row.source.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    <ExternalLink className="w-2.5 h-2.5 shrink-0" />
                    <span className="max-w-[140px] truncate">
                      {row.source.publisher}
                    </span>
                  </a>
                  <p className="text-xs text-gray-600 mt-0.5">
                    {row.source.publishedDate}
                  </p>
                </TableCell>
                <TableCell>
                  <p className="text-sm text-gray-400 max-w-xs leading-relaxed">
                    {row.equivalentPrivateConduct}
                  </p>
                </TableCell>
                <TableCell>
                  <div className="flex flex-col gap-1.5">
                    <Badge variant="outline" className="w-fit">
                      Official
                    </Badge>
                    <p className="text-xs text-gray-400 max-w-[180px] leading-relaxed">
                      {row.consequence}
                    </p>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {/* Disclaimer */}
      <p className="mt-6 text-xs text-gray-600 text-center max-w-2xl mx-auto">
        All official actions sourced from FEC filings, congressional records,
        Senate LDA database, and court documents. Equivalent conduct column is
        for contextual comparison; no legal claim is made.
      </p>
    </div>
  );
}
