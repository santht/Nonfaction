// API client with mock-first data contracts; swap to backend via BASE_URL + apiFetch.

export const BASE_URL = process.env.NEXT_PUBLIC_API_URL ?? 'http://localhost:3000/api/v1';

export interface ApiError {
  code: string;
  message: string;
  status?: number;
  details?: string;
}

export interface ApiResponse<T> {
  data?: T;
  error?: ApiError;
}

export type SourceType = 'filing' | 'news' | 'government' | 'court' | 'financial';

export interface Source {
  id: string;
  title: string;
  url: string;
  publisher: string;
  publishedDate: string;
  type: SourceType;
}

export interface Entity {
  id: string;
  name: string;
  type: 'politician' | 'lobbyist' | 'corporation' | 'nonprofit' | 'donor';
  role?: string;
  party?: string;
  state?: string;
  connectionCount: number;
  sourceCount: number;
  flagged: boolean;
  lastUpdated: string;
}

export interface Connection {
  id: string;
  fromEntity: Entity;
  toEntity: Entity;
  type: string;
  description: string;
  amount?: number;
  date: string;
  sources: Source[];
}

export interface TimingCorrelation {
  id: string;
  official: string;
  officialId: string;
  eventA: string;
  eventADate: string;
  eventB: string;
  eventBDate: string;
  daysBetween: number;
  correlationType: 'vote' | 'donation' | 'meeting' | 'regulation' | 'appointment';
  flagged: boolean;
  sources: Source[];
}

export interface ConductRow {
  id: string;
  officialAction: string;
  official: string;
  officialId: string;
  date: string;
  source: Source;
  equivalentPrivateConduct: string;
  consequence: string;
}

export interface SearchFilters {
  entityTypes?: Entity['type'][];
  dateFrom?: string;
  dateTo?: string;
  minAmount?: number;
  maxAmount?: number;
  state?: string;
  flaggedOnly?: boolean;
  query?: string;
}

export interface SubmissionData {
  entityName: string;
  connectionType: string;
  description: string;
  sourceUrl: string;
  referenceDetail: string;
  submitterNote?: string;
}

export interface Official {
  id: string;
  name: string;
  role: string;
  chamber: 'Senate' | 'House' | 'Governor';
  party: 'Democrat' | 'Republican' | 'Independent';
  state: string;
  flagged: boolean;
  connectionCount: number;
  photoUrl?: string;
}

export interface PositionHistory {
  title: string;
  startDate: string;
  endDate?: string;
}

export interface OfficialProfile {
  official: Official;
  bio: string;
  positionHistory: PositionHistory[];
  donationsReceived: number;
  pacContributions: number;
  totalFunding: number;
  votingHighlights: { bill: string; vote: 'Yea' | 'Nay' | 'Present'; date: string; note: string }[];
  timingCorrelations: TimingCorrelation[];
  conductComparisons: ConductRow[];
  relatedEntityIds: string[];
}

export interface WatchlistEntry {
  id: string;
  entityId: string;
  entityName: string;
  entityType: Entity['type'] | 'official';
  alertPreference: 'immediate' | 'daily' | 'weekly';
  createdAt: string;
}

export interface Alert {
  id: string;
  watchlistEntryId: string;
  title: string;
  severity: 'low' | 'medium' | 'high';
  createdAt: string;
  summary: string;
  sourceIds: string[];
}

export interface ContributorProfile {
  id: string;
  name: string;
  reputation: number;
  trustTier: 'Bronze' | 'Silver' | 'Gold' | 'Platinum';
  contributions: number;
  verifiedSources: number;
  recentContribution: string;
}

export interface StoryPackage {
  id: string;
  title: string;
  summary: string;
  entities: string[];
  sourceCount: number;
  dateRange: { from: string; to: string };
  updatedAt: string;
}

export interface DataSource {
  id: string;
  tier: 'Tier 1 Day One' | 'Tier 2 Week One' | 'Tier 3 Month One';
  name: string;
  description: string;
  url: string;
  dataType: string;
  updateFrequency: string;
  status: 'Active' | 'Planned' | 'Coming Soon';
}

export interface ApiEndpoint {
  path: string;
  method: 'GET' | 'POST' | 'PATCH' | 'DELETE';
  description: string;
  authRequired: boolean;
  rateLimit: string;
  requestExample?: string;
  responseExample: string;
}

export interface PlatformUpdate {
  id: string;
  title: string;
  date: string;
  category: 'release' | 'sources' | 'milestone';
  summary: string;
}

function normalizeText(input: string) {
  return input.replace(/[<>]/g, '').trim();
}

function safeDate(date?: string) {
  if (!date) return 0;
  const parsed = Date.parse(date);
  return Number.isNaN(parsed) ? 0 : parsed;
}

export async function apiFetch<T>(
  endpoint: string,
  options: RequestInit & { authToken?: string } = {}
): Promise<ApiResponse<T>> {
  const { authToken, headers, ...rest } = options;

  try {
    const response = await fetch(`${BASE_URL}${endpoint}`, {
      ...rest,
      headers: {
        'Content-Type': 'application/json',
        ...(authToken ? { Authorization: `Bearer ${authToken}` } : {}),
        ...(headers ?? {}),
      },
    });

    if (!response.ok) {
      return {
        error: {
          code: 'HTTP_ERROR',
          message: `Request failed with status ${response.status}`,
          status: response.status,
        },
      };
    }

    const data = (await response.json()) as T;
    return { data };
  } catch (err) {
    return {
      error: {
        code: 'NETWORK_ERROR',
        message: err instanceof Error ? err.message : 'Unknown network error',
      },
    };
  }
}

// Mock data --------------------------------------------------------------------

const MOCK_SOURCES: Source[] = [
  {
    id: 's1',
    title: 'FEC Filing Q3 2024 — Defense PAC Contributions',
    url: 'https://www.fec.gov/data/receipts/?committee_id=C00123456',
    publisher: 'Federal Election Commission',
    publishedDate: '2024-10-15',
    type: 'filing',
  },
  {
    id: 's2',
    title: 'Senate Vote Record SB-2024-0042: Defense Appropriations',
    url: 'https://www.congress.gov/bill/118th-congress/senate-bill/42',
    publisher: 'Congress.gov',
    publishedDate: '2024-03-22',
    type: 'government',
  },
  {
    id: 's3',
    title: 'Lobbyist Disclosure Report — Aerospace Consortium',
    url: 'https://lda.senate.gov/filings/public/filing/2024-aerospace-consortium/',
    publisher: 'U.S. Senate Lobbying Disclosure',
    publishedDate: '2024-01-20',
    type: 'filing',
  },
  {
    id: 's4',
    title: 'Campaign Finance Disclosure: Senator R. Walsh 2024',
    url: 'https://www.opensecrets.org/politicians/contrib.php?cid=N00003679',
    publisher: 'OpenSecrets',
    publishedDate: '2024-11-01',
    type: 'financial',
  },
  {
    id: 's5',
    title: 'DOJ Antitrust Investigation: PharmaCorp Merger — Case 24-cv-0091',
    url: 'https://www.courtlistener.com/docket/24-cv-0091/',
    publisher: 'CourtListener / PACER',
    publishedDate: '2024-07-08',
    type: 'court',
  },
  {
    id: 's6',
    title: "Senator Walsh's $2.4M from Defense Sector — Reuters Investigation",
    url: 'https://www.reuters.com/investigates/defense-pac-senators-2024/',
    publisher: 'Reuters',
    publishedDate: '2024-08-15',
    type: 'news',
  },
];

const MOCK_ENTITIES: Entity[] = [
  {
    id: 'e1',
    name: 'Senator Richard Walsh',
    type: 'politician',
    role: 'U.S. Senator',
    party: 'Republican',
    state: 'TX',
    connectionCount: 47,
    sourceCount: 23,
    flagged: true,
    lastUpdated: '2024-11-15',
  },
  {
    id: 'e2',
    name: 'Aerospace Defense PAC',
    type: 'corporation',
    role: 'Political Action Committee',
    connectionCount: 128,
    sourceCount: 67,
    flagged: true,
    lastUpdated: '2024-10-30',
  },
  {
    id: 'e3',
    name: 'Representative Diana Chen',
    type: 'politician',
    role: 'U.S. Representative',
    party: 'Democrat',
    state: 'CA',
    connectionCount: 31,
    sourceCount: 18,
    flagged: false,
    lastUpdated: '2024-11-10',
  },
  {
    id: 'e4',
    name: 'Marcus Leland',
    type: 'lobbyist',
    role: 'Senior Lobbyist',
    connectionCount: 89,
    sourceCount: 44,
    flagged: true,
    lastUpdated: '2024-11-01',
  },
  {
    id: 'e5',
    name: 'PharmaCorp Industries',
    type: 'corporation',
    role: 'Pharmaceutical Manufacturer',
    connectionCount: 203,
    sourceCount: 91,
    flagged: true,
    lastUpdated: '2024-11-20',
  },
  {
    id: 'e6',
    name: 'Governor Patricia Monroe',
    type: 'politician',
    role: 'State Governor',
    party: 'Democrat',
    state: 'FL',
    connectionCount: 56,
    sourceCount: 29,
    flagged: false,
    lastUpdated: '2024-10-22',
  },
  {
    id: 'e7',
    name: 'Citizens for Fair Infrastructure',
    type: 'nonprofit',
    role: 'Advocacy Nonprofit',
    connectionCount: 25,
    sourceCount: 14,
    flagged: false,
    lastUpdated: '2024-10-18',
  },
  {
    id: 'e8',
    name: 'Elena Ross',
    type: 'donor',
    role: 'Private Donor',
    connectionCount: 17,
    sourceCount: 9,
    flagged: true,
    lastUpdated: '2024-11-08',
  },
];

const MOCK_TIMING: TimingCorrelation[] = [
  {
    id: 't1',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    eventA: 'Received $450,000 from Aerospace Defense PAC',
    eventADate: '2024-02-14',
    eventB: 'Voted YES on SB-2024-0042 ($8.2B Defense Appropriations)',
    eventBDate: '2024-03-22',
    daysBetween: 37,
    correlationType: 'donation',
    flagged: true,
    sources: [MOCK_SOURCES[0], MOCK_SOURCES[1]],
  },
  {
    id: 't2',
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    eventA: 'Private meeting with PharmaCorp CEO (LDA disclosure)',
    eventADate: '2024-05-10',
    eventB: 'Co-sponsored HR-2024-0189: Drug Price Deregulation Act',
    eventBDate: '2024-05-28',
    daysBetween: 18,
    correlationType: 'meeting',
    flagged: true,
    sources: [MOCK_SOURCES[2]],
  },
  {
    id: 't3',
    official: 'Gov. Patricia Monroe',
    officialId: 'e6',
    eventA: 'Marcus Leland bundled $1.2M for Monroe campaign',
    eventADate: '2023-10-01',
    eventB: "Monroe appointed Leland's client as State Treasurer",
    eventBDate: '2024-01-15',
    daysBetween: 106,
    correlationType: 'appointment',
    flagged: true,
    sources: [MOCK_SOURCES[3]],
  },
  {
    id: 't4',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    eventA: 'Attended Aerospace Consortium annual gala',
    eventADate: '2024-06-05',
    eventB: 'Blocked floor vote on defense contractor oversight amendment',
    eventBDate: '2024-06-19',
    daysBetween: 14,
    correlationType: 'vote',
    flagged: true,
    sources: [MOCK_SOURCES[5]],
  },
  {
    id: 't5',
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    eventA: 'Town hall on healthcare access (public record)',
    eventADate: '2024-09-12',
    eventB: 'Voted YES on Medicare expansion amendment',
    eventBDate: '2024-09-30',
    daysBetween: 18,
    correlationType: 'vote',
    flagged: false,
    sources: [MOCK_SOURCES[1]],
  },
];

const MOCK_CONDUCT: ConductRow[] = [
  {
    id: 'c1',
    officialAction:
      'Accepted $450,000 PAC donation from defense contractor, then voted to award same contractor $8.2B contract',
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    date: '2024-03-22',
    source: MOCK_SOURCES[0],
    equivalentPrivateConduct:
      'Receiving payment from a party and then awarding them a contract in a professional capacity',
    consequence: 'No investigation. Reelected. Contractor received full award.',
  },
  {
    id: 'c2',
    officialAction:
      "Met privately with pharmaceutical CEO, then co-sponsored bill removing drug pricing controls affecting CEO's company",
    official: 'Rep. Diana Chen',
    officialId: 'e3',
    date: '2024-05-28',
    source: MOCK_SOURCES[2],
    equivalentPrivateConduct: 'Undisclosed conflict of interest in regulatory decision-making',
    consequence: 'Bill passed committee. Under ethics review.',
  },
  {
    id: 'c3',
    officialAction:
      "Appointed major donor's business associate to cabinet position without competitive process",
    official: 'Gov. Patricia Monroe',
    officialId: 'e6',
    date: '2024-01-15',
    source: MOCK_SOURCES[3],
    equivalentPrivateConduct: 'Nepotism / quid pro quo employment arrangement',
    consequence: 'Appointee confirmed. FOIA request filed. Ongoing public scrutiny.',
  },
  {
    id: 'c4',
    officialAction:
      "Used official government travel budget for 14 trips to lobbying firm's headquarter city",
    official: 'Sen. Richard Walsh',
    officialId: 'e1',
    date: '2024-08-01',
    source: MOCK_SOURCES[5],
    equivalentPrivateConduct: 'Misappropriation of company funds for personal/external benefit',
    consequence: 'GAO inquiry requested. No disciplinary action taken.',
  },
];

const MOCK_OFFICIALS: Official[] = [
  {
    id: 'o1',
    name: 'Senator Richard Walsh',
    role: 'U.S. Senator',
    chamber: 'Senate',
    party: 'Republican',
    state: 'TX',
    flagged: true,
    connectionCount: 47,
  },
  {
    id: 'o2',
    name: 'Representative Diana Chen',
    role: 'U.S. Representative',
    chamber: 'House',
    party: 'Democrat',
    state: 'CA',
    flagged: true,
    connectionCount: 31,
  },
  {
    id: 'o3',
    name: 'Governor Patricia Monroe',
    role: 'State Governor',
    chamber: 'Governor',
    party: 'Democrat',
    state: 'FL',
    flagged: false,
    connectionCount: 56,
  },
  {
    id: 'o4',
    name: 'Senator Joseph Alvarez',
    role: 'U.S. Senator',
    chamber: 'Senate',
    party: 'Independent',
    state: 'NM',
    flagged: false,
    connectionCount: 22,
  },
  {
    id: 'o5',
    name: 'Representative Kendra Price',
    role: 'U.S. Representative',
    chamber: 'House',
    party: 'Republican',
    state: 'OH',
    flagged: true,
    connectionCount: 39,
  },
  {
    id: 'o6',
    name: 'Representative Malik Grant',
    role: 'U.S. Representative',
    chamber: 'House',
    party: 'Democrat',
    state: 'IL',
    flagged: false,
    connectionCount: 19,
  },
];

const MOCK_OFFICIAL_PROFILES: Record<string, OfficialProfile> = {
  o1: {
    official: MOCK_OFFICIALS[0],
    bio: 'Serves on Armed Services and Appropriations committees. Tracking includes campaign finance, vote records, and disclosed meetings.',
    positionHistory: [
      { title: 'U.S. Senator (TX)', startDate: '2018-01-03' },
      { title: 'Texas State Senator', startDate: '2011-01-11', endDate: '2017-12-20' },
    ],
    donationsReceived: 2400000,
    pacContributions: 1130000,
    totalFunding: 5100000,
    votingHighlights: [
      { bill: 'SB-2024-0042', vote: 'Yea', date: '2024-03-22', note: 'Defense appropriations package.' },
      { bill: 'SB-2024-0811', vote: 'Nay', date: '2024-08-11', note: 'Contractor oversight amendment.' },
    ],
    timingCorrelations: MOCK_TIMING.filter((t) => t.officialId === 'e1'),
    conductComparisons: MOCK_CONDUCT.filter((c) => c.officialId === 'e1'),
    relatedEntityIds: ['e2', 'e4', 'e5'],
  },
  o2: {
    official: MOCK_OFFICIALS[1],
    bio: 'House member focused on health policy and commerce. Profile aggregates public disclosures and vote metadata.',
    positionHistory: [
      { title: 'U.S. Representative (CA-14)', startDate: '2021-01-03' },
      { title: 'California State Assembly', startDate: '2015-01-05', endDate: '2020-12-15' },
    ],
    donationsReceived: 1300000,
    pacContributions: 470000,
    totalFunding: 2900000,
    votingHighlights: [
      { bill: 'HR-2024-0189', vote: 'Yea', date: '2024-05-28', note: 'Drug pricing deregulation proposal.' },
      { bill: 'HR-2024-0622', vote: 'Yea', date: '2024-09-30', note: 'Medicare expansion amendment.' },
    ],
    timingCorrelations: MOCK_TIMING.filter((t) => t.officialId === 'e3'),
    conductComparisons: MOCK_CONDUCT.filter((c) => c.officialId === 'e3'),
    relatedEntityIds: ['e5', 'e7'],
  },
};

const MOCK_WATCHLIST: WatchlistEntry[] = [
  {
    id: 'w1',
    entityId: 'e1',
    entityName: 'Senator Richard Walsh',
    entityType: 'politician',
    alertPreference: 'immediate',
    createdAt: '2025-01-10',
  },
  {
    id: 'w2',
    entityId: 'e5',
    entityName: 'PharmaCorp Industries',
    entityType: 'corporation',
    alertPreference: 'daily',
    createdAt: '2025-01-17',
  },
];

const MOCK_ALERTS: Alert[] = [
  {
    id: 'a1',
    watchlistEntryId: 'w1',
    title: 'New filing linked to defense PAC disbursement',
    severity: 'high',
    createdAt: '2025-01-21',
    summary: 'A new FEC filing references disbursements connected to previously flagged entities.',
    sourceIds: ['s1'],
  },
  {
    id: 'a2',
    watchlistEntryId: 'w2',
    title: 'Updated court docket in merger case',
    severity: 'medium',
    createdAt: '2025-01-18',
    summary: 'CourtListener docket entry updated with hearing schedule changes.',
    sourceIds: ['s5'],
  },
];

const MOCK_CONTRIBUTORS: ContributorProfile[] = [
  {
    id: 'u1',
    name: 'Alex Romero',
    reputation: 9840,
    trustTier: 'Platinum',
    contributions: 422,
    verifiedSources: 389,
    recentContribution: 'Submitted three validated FEC filing links',
  },
  {
    id: 'u2',
    name: 'Jordan Lee',
    reputation: 8120,
    trustTier: 'Gold',
    contributions: 305,
    verifiedSources: 272,
    recentContribution: 'Added state procurement records for oversight package',
  },
  {
    id: 'u3',
    name: 'Sam Patel',
    reputation: 6330,
    trustTier: 'Silver',
    contributions: 187,
    verifiedSources: 154,
    recentContribution: 'Cited congressional hearing transcript source set',
  },
];

const MOCK_STORY_PACKAGES: StoryPackage[] = [
  {
    id: 'sp1',
    title: 'Defense Appropriations Timing Chain',
    summary: 'Donations, meetings, and vote chronology around SB-2024-0042.',
    entities: ['Senator Richard Walsh', 'Aerospace Defense PAC', 'Marcus Leland'],
    sourceCount: 34,
    dateRange: { from: '2023-12-01', to: '2024-08-20' },
    updatedAt: '2025-01-20',
  },
  {
    id: 'sp2',
    title: 'Healthcare Deregulation Influence Package',
    summary: 'Meeting logs, bill sponsorship trail, and campaign financing records.',
    entities: ['Representative Diana Chen', 'PharmaCorp Industries'],
    sourceCount: 26,
    dateRange: { from: '2024-03-01', to: '2024-10-05' },
    updatedAt: '2025-01-11',
  },
  {
    id: 'sp3',
    title: 'State Appointment Patronage Signals',
    summary: 'Bundled donations and appointment timeline in state executive office.',
    entities: ['Governor Patricia Monroe', 'Marcus Leland'],
    sourceCount: 19,
    dateRange: { from: '2023-09-01', to: '2024-04-10' },
    updatedAt: '2025-01-08',
  },
];

const SOURCE_CATALOG_NAMES = [
  'FEC Receipts API',
  'FEC Disbursements API',
  'Congress.gov Bill Status',
  'Congress Roll Call Votes',
  'Senate LDA Filings',
  'House Clerk Disclosure',
  'CourtListener Dockets',
  'PACER Federal Cases',
  'OpenSecrets Donations',
  'State Ethics Filings',
  'SEC EDGAR 8-K',
  'Municipal Contract Awards',
];

const MOCK_DATA_SOURCES: DataSource[] = Array.from({ length: 108 }, (_, idx) => {
  const tier = idx < 36 ? 'Tier 1 Day One' : idx < 72 ? 'Tier 2 Week One' : 'Tier 3 Month One';
  const status = idx < 52 ? 'Active' : idx < 88 ? 'Planned' : 'Coming Soon';
  const baseName = SOURCE_CATALOG_NAMES[idx % SOURCE_CATALOG_NAMES.length];
  return {
    id: `ds${idx + 1}`,
    tier,
    name: `${baseName} ${Math.floor(idx / SOURCE_CATALOG_NAMES.length) + 1}`,
    description: 'Public records source ingested for structured accountability analysis and source-linked evidence.',
    url: `https://data.example.org/source/${idx + 1}`,
    dataType: idx % 2 === 0 ? 'Financial disclosure' : 'Legislative / legal records',
    updateFrequency: idx % 3 === 0 ? 'Daily' : idx % 3 === 1 ? 'Weekly' : 'Monthly',
    status,
  };
});

const MOCK_API_ENDPOINTS: ApiEndpoint[] = [
  {
    path: '/entities',
    method: 'GET',
    description: 'List and filter entities by type, state, and flag status.',
    authRequired: false,
    rateLimit: '120 requests/minute',
    requestExample: 'GET /api/v1/entities?type=politician&state=TX',
    responseExample: '{"data":[{"id":"e1","name":"Senator Richard Walsh"}]}'
  },
  {
    path: '/officials/:id',
    method: 'GET',
    description: 'Get full profile details for one official.',
    authRequired: false,
    rateLimit: '120 requests/minute',
    requestExample: 'GET /api/v1/officials/o1',
    responseExample: '{"data":{"official":{"id":"o1"},"timingCorrelations":[]}}'
  },
  {
    path: '/submissions',
    method: 'POST',
    description: 'Submit a sourced connection for review.',
    authRequired: true,
    rateLimit: '30 requests/minute',
    requestExample: 'POST /api/v1/submissions',
    responseExample: '{"data":{"success":true,"id":"sub_1738181"}}'
  },
  {
    path: '/watchlist',
    method: 'GET',
    description: 'Retrieve user watchlist entries and alert preferences.',
    authRequired: true,
    rateLimit: '60 requests/minute',
    responseExample: '{"data":[{"id":"w1","entityName":"Senator Richard Walsh"}]}'
  },
];

const MOCK_UPDATES: PlatformUpdate[] = [
  {
    id: 'up1',
    title: 'Tier 1 source ingest pipeline launched',
    date: '2025-01-25',
    category: 'milestone',
    summary: 'Enabled daily sync for baseline federal filing and legislative datasets.',
  },
  {
    id: 'up2',
    title: 'Story package exports added',
    date: '2025-01-20',
    category: 'release',
    summary: 'Users can export ZIP bundles containing records, timelines, and source manifests.',
  },
  {
    id: 'up3',
    title: '18 new court dockets indexed',
    date: '2025-01-14',
    category: 'sources',
    summary: 'Expanded court coverage for antitrust and procurement litigation tracking.',
  },
];

// API functions ----------------------------------------------------------------

export async function searchEntities(query: string, filters: SearchFilters = {}): Promise<Entity[]> {
  await new Promise((resolve) => setTimeout(resolve, 150));

  let results = [...MOCK_ENTITIES];
  const normalizedQuery = normalizeText(query).toLowerCase();

  if (normalizedQuery) {
    results = results.filter(
      (entity) =>
        entity.name.toLowerCase().includes(normalizedQuery) ||
        entity.role?.toLowerCase().includes(normalizedQuery) ||
        entity.type.toLowerCase().includes(normalizedQuery)
    );
  }

  if (filters.entityTypes?.length) {
    results = results.filter((entity) => filters.entityTypes?.includes(entity.type));
  }

  if (filters.state) {
    results = results.filter((entity) => entity.state === filters.state);
  }

  if (filters.flaggedOnly) {
    results = results.filter((entity) => entity.flagged);
  }

  if (filters.dateFrom || filters.dateTo) {
    const from = safeDate(filters.dateFrom);
    const to = safeDate(filters.dateTo) || Number.MAX_SAFE_INTEGER;
    results = results.filter((entity) => {
      const updated = safeDate(entity.lastUpdated);
      return updated >= from && updated <= to;
    });
  }

  if (filters.minAmount || filters.maxAmount) {
    const min = filters.minAmount ?? 0;
    const max = filters.maxAmount ?? Number.MAX_SAFE_INTEGER;
    results = results.filter((entity) => entity.connectionCount * 10000 >= min && entity.connectionCount * 10000 <= max);
  }

  return results;
}

export async function getEntity(id: string): Promise<Entity | null> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  return MOCK_ENTITIES.find((entity) => entity.id === id) ?? null;
}

export async function getEntityConnections(entityId: string): Promise<Connection[]> {
  await new Promise((resolve) => setTimeout(resolve, 100));
  const entity = MOCK_ENTITIES.find((item) => item.id === entityId);
  if (!entity) return [];

  return [
    {
      id: 'conn1',
      fromEntity: entity,
      toEntity: MOCK_ENTITIES[1],
      type: 'financial',
      description: '$450,000 PAC contribution via Aerospace Defense PAC',
      amount: 450000,
      date: '2024-02-14',
      sources: [MOCK_SOURCES[0]],
    },
    {
      id: 'conn2',
      fromEntity: entity,
      toEntity: MOCK_ENTITIES[3],
      type: 'meeting',
      description: 'Recorded meeting — Senate lobbying disclosure database',
      amount: 0,
      date: '2024-06-05',
      sources: [MOCK_SOURCES[2]],
    },
    {
      id: 'conn3',
      fromEntity: entity,
      toEntity: MOCK_ENTITIES[4],
      type: 'regulatory',
      description: 'Post-hearing correspondence on oversight amendment language',
      amount: 220000,
      date: '2024-06-18',
      sources: [MOCK_SOURCES[5]],
    },
  ];
}

export async function getTimingCorrelations(filters: {
  flaggedOnly?: boolean;
  officialId?: string;
} = {}): Promise<TimingCorrelation[]> {
  await new Promise((resolve) => setTimeout(resolve, 120));
  let rows = [...MOCK_TIMING];

  if (filters.flaggedOnly) {
    rows = rows.filter((row) => row.flagged);
  }

  if (filters.officialId) {
    rows = rows.filter((row) => row.officialId === filters.officialId);
  }

  return rows;
}

export async function getConductRows(filters: { officialId?: string } = {}): Promise<ConductRow[]> {
  await new Promise((resolve) => setTimeout(resolve, 120));
  if (filters.officialId) {
    return MOCK_CONDUCT.filter((row) => row.officialId === filters.officialId);
  }
  return MOCK_CONDUCT;
}

export async function getStats(): Promise<{
  entities: number;
  connections: number;
  sources: number;
  flagged: number;
}> {
  await new Promise((resolve) => setTimeout(resolve, 60));
  return { entities: 2847, connections: 14203, sources: 8916, flagged: 342 };
}

export async function submitConnection(
  data: SubmissionData
): Promise<{ success: boolean; id?: string; error?: string }> {
  await new Promise((resolve) => setTimeout(resolve, 320));

  const clean: SubmissionData = {
    ...data,
    entityName: normalizeText(data.entityName),
    connectionType: normalizeText(data.connectionType),
    description: normalizeText(data.description),
    sourceUrl: data.sourceUrl.trim(),
    referenceDetail: normalizeText(data.referenceDetail),
    submitterNote: data.submitterNote ? normalizeText(data.submitterNote) : undefined,
  };

  if (!clean.sourceUrl || !clean.referenceDetail || !clean.entityName) {
    return { success: false, error: 'Entity, source URL, and reference detail are required.' };
  }

  return { success: true, id: `sub_${Date.now()}` };
}

export async function getOfficials(filters: {
  state?: string;
  party?: Official['party'];
  chamber?: Official['chamber'];
  flaggedOnly?: boolean;
} = {}): Promise<Official[]> {
  await new Promise((resolve) => setTimeout(resolve, 120));

  return MOCK_OFFICIALS.filter((official) => {
    if (filters.state && official.state !== filters.state) return false;
    if (filters.party && official.party !== filters.party) return false;
    if (filters.chamber && official.chamber !== filters.chamber) return false;
    if (filters.flaggedOnly && !official.flagged) return false;
    return true;
  });
}

export async function getOfficialProfile(id: string): Promise<OfficialProfile | null> {
  await new Promise((resolve) => setTimeout(resolve, 150));
  return MOCK_OFFICIAL_PROFILES[id] ?? null;
}

export async function getWatchlistEntries(): Promise<WatchlistEntry[]> {
  await new Promise((resolve) => setTimeout(resolve, 70));
  return MOCK_WATCHLIST;
}

export async function getAlerts(): Promise<Alert[]> {
  await new Promise((resolve) => setTimeout(resolve, 70));
  return MOCK_ALERTS;
}

export async function getContributorLeaderboard(): Promise<ContributorProfile[]> {
  await new Promise((resolve) => setTimeout(resolve, 70));
  return MOCK_CONTRIBUTORS;
}

export async function getStoryPackages(query?: string): Promise<StoryPackage[]> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  const q = query ? normalizeText(query).toLowerCase() : '';
  if (!q) return MOCK_STORY_PACKAGES;
  return MOCK_STORY_PACKAGES.filter(
    (item) => item.title.toLowerCase().includes(q) || item.entities.some((entity) => entity.toLowerCase().includes(q))
  );
}

export async function getDataSources(): Promise<DataSource[]> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  return MOCK_DATA_SOURCES;
}

export async function getApiEndpoints(): Promise<ApiEndpoint[]> {
  await new Promise((resolve) => setTimeout(resolve, 40));
  return MOCK_API_ENDPOINTS;
}

export async function getRecentActivity(): Promise<
  { id: string; title: string; time: string; source: string; severity: 'low' | 'medium' | 'high' }[]
> {
  await new Promise((resolve) => setTimeout(resolve, 75));
  return [
    { id: 'ra1', title: 'New timing correlation flagged for SB-2024-0042', time: '2h ago', source: 'Congress.gov', severity: 'high' },
    { id: 'ra2', title: 'Court docket update linked to PharmaCorp profile', time: '8h ago', source: 'CourtListener', severity: 'medium' },
    { id: 'ra3', title: 'Contributor submission verified and published', time: '1d ago', source: 'Internal review', severity: 'low' },
  ];
}

export async function getPlatformUpdates(): Promise<PlatformUpdate[]> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  return MOCK_UPDATES;
}

export async function getSourceByIds(ids: string[]): Promise<Source[]> {
  await new Promise((resolve) => setTimeout(resolve, 40));
  return MOCK_SOURCES.filter((source) => ids.includes(source.id));
}

export async function getRelatedEntities(entityIds: string[]): Promise<Entity[]> {
  await new Promise((resolve) => setTimeout(resolve, 80));
  return MOCK_ENTITIES.filter((entity) => entityIds.includes(entity.id));
}

export async function getAllSources(): Promise<Source[]> {
  await new Promise((resolve) => setTimeout(resolve, 20));
  return MOCK_SOURCES;
}
