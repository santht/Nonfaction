'use client';

import { useSearchParams, useRouter } from 'next/navigation';
import { useState, useEffect, Suspense } from 'react';
import Link from 'next/link';
import {
  AlertTriangle,
  ExternalLink,
  Filter,
  User,
  Building2,
  Users,
  Heart,
  DollarSign,
} from 'lucide-react';
import { searchEntities, type Entity, type SearchFilters } from '@/lib/api';
import { SearchInput } from '@/components/ui/SearchInput';
import { Card, CardContent } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';

const ENTITY_TYPES: { value: Entity['type']; label: string; icon: React.ElementType }[] = [
  { value: 'politician', label: 'Politicians', icon: User },
  { value: 'corporation', label: 'Corporations', icon: Building2 },
  { value: 'lobbyist', label: 'Lobbyists', icon: Users },
  { value: 'nonprofit', label: 'Nonprofits', icon: Heart },
  { value: 'donor', label: 'Donors', icon: DollarSign },
];

const TYPE_BADGE: Record<Entity['type'], { variant: 'blue' | 'default' | 'yellow' | 'green' | 'red'; label: string }> = {
  politician: { variant: 'blue', label: 'Politician' },
  corporation: { variant: 'default', label: 'Corporation' },
  lobbyist: { variant: 'yellow', label: 'Lobbyist' },
  nonprofit: { variant: 'green', label: 'Nonprofit' },
  donor: { variant: 'default', label: 'Donor' },
};

function SearchResults() {
  const searchParams = useSearchParams();
  const router = useRouter();

  const [query, setQuery] = useState(searchParams.get('q') ?? '');
  const [results, setResults] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(false);
  const [selectedTypes, setSelectedTypes] = useState<Entity['type'][]>([]);
  const [flaggedOnly, setFlaggedOnly] = useState(false);

  useEffect(() => {
    const q = searchParams.get('q') ?? '';
    setQuery(q);
    doSearch(q, selectedTypes, flaggedOnly);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [searchParams]);

  async function doSearch(
    q: string,
    types: Entity['type'][],
    flagged: boolean
  ) {
    setLoading(true);
    const filters: SearchFilters = {
      entityTypes: types.length ? types : undefined,
      flaggedOnly: flagged || undefined,
    };
    const res = await searchEntities(q, filters);
    setResults(res);
    setLoading(false);
  }

  function handleSearch(val: string) {
    router.push(`/search?q=${encodeURIComponent(val)}`);
  }

  function toggleType(type: Entity['type']) {
    const next = selectedTypes.includes(type)
      ? selectedTypes.filter((t) => t !== type)
      : [...selectedTypes, type];
    setSelectedTypes(next);
    doSearch(query, next, flaggedOnly);
  }

  function toggleFlagged() {
    const next = !flaggedOnly;
    setFlaggedOnly(next);
    doSearch(query, selectedTypes, next);
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 py-10">
      <div className="mb-8">
        <SearchInput
          large
          placeholder="Search politicians, corporations, lobbyists, donors…"
          defaultValue={query}
          onSearch={handleSearch}
          containerClassName="max-w-2xl"
        />
      </div>

      <div className="flex flex-col md:flex-row gap-8">
        {/* Sidebar */}
        <aside className="w-full md:w-64 shrink-0">
          <div className="sticky top-20">
            <div className="flex items-center gap-2 mb-4">
              <Filter className="w-4 h-4 text-gray-500" />
              <span className="text-sm font-semibold text-gray-300">
                Filters
              </span>
            </div>

            {/* Entity type */}
            <div className="mb-6">
              <h4 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">
                Entity Type
              </h4>
              <div className="space-y-1.5">
                {ENTITY_TYPES.map(({ value, label, icon: Icon }) => (
                  <button
                    key={value}
                    onClick={() => toggleType(value)}
                    className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors ${
                      selectedTypes.includes(value)
                        ? 'bg-blue-500/15 text-blue-400 border border-blue-500/20'
                        : 'text-gray-400 hover:bg-white/6 hover:text-white border border-transparent'
                    }`}
                  >
                    <Icon className="w-3.5 h-3.5" />
                    {label}
                  </button>
                ))}
              </div>
            </div>

            {/* Flagged filter */}
            <div className="mb-6">
              <h4 className="text-xs font-semibold text-gray-500 uppercase tracking-wider mb-3">
                Status
              </h4>
              <button
                onClick={toggleFlagged}
                className={`w-full flex items-center gap-2.5 px-3 py-2 rounded-lg text-sm transition-colors border ${
                  flaggedOnly
                    ? 'bg-red-500/15 text-red-400 border-red-500/20'
                    : 'text-gray-400 hover:bg-white/6 hover:text-white border-transparent'
                }`}
              >
                <AlertTriangle className="w-3.5 h-3.5" />
                Flagged Only
              </button>
            </div>

            {(selectedTypes.length > 0 || flaggedOnly) && (
              <Button
                variant="ghost"
                size="sm"
                onClick={() => {
                  setSelectedTypes([]);
                  setFlaggedOnly(false);
                  doSearch(query, [], false);
                }}
                className="w-full text-xs"
              >
                Clear filters
              </Button>
            )}
          </div>
        </aside>

        {/* Results */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between mb-4">
            <p className="text-sm text-gray-500">
              {loading ? (
                'Searching…'
              ) : (
                <>
                  <span className="text-white font-medium">{results.length}</span>{' '}
                  {results.length === 1 ? 'result' : 'results'}
                  {query && (
                    <>
                      {' '}
                      for{' '}
                      <span className="text-white font-medium">
                        &quot;{query}&quot;
                      </span>
                    </>
                  )}
                </>
              )}
            </p>
          </div>

          {loading ? (
            <div className="space-y-3">
              {[1, 2, 3].map((i) => (
                <div
                  key={i}
                  className="h-28 rounded-xl bg-white/4 border border-white/6 animate-pulse"
                />
              ))}
            </div>
          ) : results.length === 0 ? (
            <div className="text-center py-20 text-gray-500">
              <p className="text-lg mb-2">No entities found</p>
              <p className="text-sm">
                Try adjusting your search or clearing filters
              </p>
            </div>
          ) : (
            <div className="space-y-3">
              {results.map((entity) => (
                <Link key={entity.id} href={`/entity/${entity.id}`}>
                  <Card hover className="p-5">
                    <CardContent className="p-0">
                      <div className="flex items-start justify-between gap-4">
                        <div className="flex-1 min-w-0">
                          <div className="flex flex-wrap items-center gap-2 mb-1">
                            <span className="font-semibold text-white">
                              {entity.name}
                            </span>
                            <Badge variant={TYPE_BADGE[entity.type].variant}>
                              {TYPE_BADGE[entity.type].label}
                            </Badge>
                            {entity.flagged && (
                              <Badge variant="red">
                                <AlertTriangle className="w-2.5 h-2.5" />
                                Flagged
                              </Badge>
                            )}
                          </div>
                          {entity.role && (
                            <p className="text-sm text-gray-400">
                              {entity.role}
                              {entity.state && ` · ${entity.state}`}
                              {entity.party && ` · ${entity.party}`}
                            </p>
                          )}
                          <p className="text-xs text-gray-600 mt-2">
                            Updated {entity.lastUpdated}
                          </p>
                        </div>
                        <div className="flex gap-3 shrink-0 text-right">
                          <div>
                            <div className="text-lg font-bold text-white">
                              {entity.connectionCount}
                            </div>
                            <div className="text-xs text-gray-500">
                              connections
                            </div>
                          </div>
                          <div>
                            <div className="text-lg font-bold text-blue-400 flex items-center gap-1 justify-end">
                              <ExternalLink className="w-3 h-3" />
                              {entity.sourceCount}
                            </div>
                            <div className="text-xs text-gray-500">sources</div>
                          </div>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                </Link>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

export default function SearchPage() {
  return (
    <Suspense
      fallback={
        <div className="max-w-7xl mx-auto px-4 py-10">
          <div className="h-14 bg-white/4 rounded-xl animate-pulse mb-8 max-w-2xl" />
        </div>
      }
    >
      <SearchResults />
    </Suspense>
  );
}
