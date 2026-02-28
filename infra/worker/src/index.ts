/**
 * Nonfaction API Gateway — Cloudflare Worker
 *
 * Routes:
 *   GET  /api/v1/search?q=...     → Search entities (D1)
 *   GET  /api/v1/entities/:id     → Get entity by ID (D1)
 *   GET  /api/v1/entities         → List entities (D1, paginated)
 *   GET  /api/v1/graph/:id/neighbors → Entity neighbors
 *   GET  /health                  → Health check
 *   GET  /ready                   → Readiness (D1 connectivity)
 *   GET  /                        → Landing page redirect
 */

export interface Env {
	DB: D1Database;
	ENVIRONMENT: string;
	API_VERSION: string;
}

// ─── CORS headers ─────────────────────────────────────────────────────────────

const CORS_HEADERS = {
	'Access-Control-Allow-Origin': '*',
	'Access-Control-Allow-Methods': 'GET, POST, OPTIONS',
	'Access-Control-Allow-Headers': 'Content-Type, Authorization',
};

function corsResponse(body: string | object | null, status = 200) {
	const json = typeof body === 'string' ? body : JSON.stringify(body);
	return new Response(json, {
		status,
		headers: {
			'Content-Type': 'application/json',
			...CORS_HEADERS,
		},
	});
}

// ─── Router ───────────────────────────────────────────────────────────────────

export default {
	async fetch(request: Request, env: Env): Promise<Response> {
		const url = new URL(request.url);
		const path = url.pathname;

		// CORS preflight
		if (request.method === 'OPTIONS') {
			return new Response(null, { status: 204, headers: CORS_HEADERS });
		}

		try {
			// Health check
			if (path === '/health') {
				return corsResponse({ status: 'ok', version: env.API_VERSION });
			}

			// Readiness check
			if (path === '/ready') {
				try {
					await env.DB.prepare('SELECT 1').first();
					return corsResponse({ status: 'ready', database: 'connected' });
				} catch (e: any) {
					return corsResponse({ status: 'not ready', database: e.message }, 503);
				}
			}

			// Search
			if (path === '/api/v1/search' && request.method === 'GET') {
				return handleSearch(url, env);
			}

			// Get entity by ID
			const entityMatch = path.match(/^\/api\/v1\/entities\/([0-9a-f-]{36})$/);
			if (entityMatch && request.method === 'GET') {
				return handleGetEntity(entityMatch[1], env);
			}

			// List entities
			if (path === '/api/v1/entities' && request.method === 'GET') {
				return handleListEntities(url, env);
			}

			// Entity neighbors (graph)
			const neighborMatch = path.match(/^\/api\/v1\/graph\/([0-9a-f-]{36})\/neighbors$/);
			if (neighborMatch && request.method === 'GET') {
				return handleNeighbors(neighborMatch[1], env);
			}

			// Root redirect
			if (path === '/' || path === '') {
				return corsResponse({
					name: 'Nonfaction API',
					version: env.API_VERSION,
					docs: '/api/v1/docs',
					health: '/health',
				});
			}

			return corsResponse({ error: { code: 'NOT_FOUND', message: `No route for ${path}` } }, 404);
		} catch (e: any) {
			console.error('Unhandled error:', e);
			return corsResponse(
				{ error: { code: 'INTERNAL_ERROR', message: 'An internal error occurred' } },
				500
			);
		}
	},
};

// ─── Handlers ─────────────────────────────────────────────────────────────────

async function handleSearch(url: URL, env: Env): Promise<Response> {
	const query = url.searchParams.get('q');
	if (!query || query.trim().length === 0) {
		return corsResponse({ error: { code: 'BAD_REQUEST', message: "query parameter 'q' must not be empty" } }, 400);
	}

	const page = Math.max(1, parseInt(url.searchParams.get('page') || '1'));
	const perPage = Math.min(100, Math.max(1, parseInt(url.searchParams.get('per_page') || '20')));
	const entityType = url.searchParams.get('type');
	const offset = (page - 1) * perPage;

	let sql = `SELECT id, entity_type, data FROM entities WHERE data LIKE ?`;
	const params: any[] = [`%${query}%`];

	if (entityType) {
		sql += ` AND entity_type = ?`;
		params.push(entityType);
	}

	sql += ` ORDER BY created_at DESC LIMIT ? OFFSET ?`;
	params.push(perPage, offset);

	const results = await env.DB.prepare(sql).bind(...params).all();

	return corsResponse({
		query,
		page,
		per_page: perPage,
		total_results: results.results.length,
		results: results.results.map((row: any) => ({
			entity_id: row.id,
			entity_type: row.entity_type,
			data: JSON.parse(row.data),
		})),
	});
}

async function handleGetEntity(id: string, env: Env): Promise<Response> {
	const row = await env.DB.prepare('SELECT id, entity_type, data, version, created_at, updated_at FROM entities WHERE id = ?')
		.bind(id)
		.first();

	if (!row) {
		return corsResponse({ error: { code: 'NOT_FOUND', message: `entity ${id}` } }, 404);
	}

	return corsResponse({
		id: row.id,
		entity_type: row.entity_type,
		data: JSON.parse(row.data as string),
		version: row.version,
		created_at: row.created_at,
		updated_at: row.updated_at,
	});
}

async function handleListEntities(url: URL, env: Env): Promise<Response> {
	const page = Math.max(1, parseInt(url.searchParams.get('page') || '1'));
	const perPage = Math.min(100, Math.max(1, parseInt(url.searchParams.get('per_page') || '20')));
	const entityType = url.searchParams.get('type');
	const offset = (page - 1) * perPage;

	let countSql = 'SELECT COUNT(*) as total FROM entities';
	let listSql = 'SELECT id, entity_type, data, version, created_at, updated_at FROM entities';
	const params: any[] = [];

	if (entityType) {
		countSql += ' WHERE entity_type = ?';
		listSql += ' WHERE entity_type = ?';
		params.push(entityType);
	}

	listSql += ' ORDER BY created_at DESC LIMIT ? OFFSET ?';

	const countResult = await env.DB.prepare(countSql).bind(...params).first<{ total: number }>();
	const totalCount = countResult?.total || 0;

	const results = await env.DB.prepare(listSql).bind(...params, perPage, offset).all();

	return corsResponse({
		page,
		per_page: perPage,
		total_count: totalCount,
		total_pages: Math.ceil(totalCount / perPage),
		items: results.results.map((row: any) => ({
			id: row.id,
			entity_type: row.entity_type,
			data: JSON.parse(row.data),
			version: row.version,
			created_at: row.created_at,
			updated_at: row.updated_at,
		})),
	});
}

async function handleNeighbors(id: string, env: Env): Promise<Response> {
	// Check entity exists
	const entity = await env.DB.prepare('SELECT id FROM entities WHERE id = ?').bind(id).first();
	if (!entity) {
		return corsResponse({ error: { code: 'NOT_FOUND', message: `entity ${id}` } }, 404);
	}

	const outgoing = await env.DB.prepare(
		'SELECT r.id as rel_id, r.rel_type, r.data as rel_data, e.id as entity_id, e.entity_type, e.data as entity_data ' +
		'FROM relationships r JOIN entities e ON r.to_entity = e.id WHERE r.from_entity = ?'
	).bind(id).all();

	const incoming = await env.DB.prepare(
		'SELECT r.id as rel_id, r.rel_type, r.data as rel_data, e.id as entity_id, e.entity_type, e.data as entity_data ' +
		'FROM relationships r JOIN entities e ON r.from_entity = e.id WHERE r.to_entity = ?'
	).bind(id).all();

	return corsResponse({
		entity_id: id,
		outgoing: outgoing.results.map((r: any) => ({
			relationship_id: r.rel_id,
			relationship_type: r.rel_type,
			target_id: r.entity_id,
			target_type: r.entity_type,
			target_data: JSON.parse(r.entity_data),
		})),
		incoming: incoming.results.map((r: any) => ({
			relationship_id: r.rel_id,
			relationship_type: r.rel_type,
			source_id: r.entity_id,
			source_type: r.entity_type,
			source_data: JSON.parse(r.entity_data),
		})),
	});
}
