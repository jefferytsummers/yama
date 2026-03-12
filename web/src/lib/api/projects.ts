/**
 * Project API client
 * CRUD operations for projects and libraries
 */

import { apiClient } from './client';

export interface ProjectConfig {
	enabled_tools: string[];
	default_model?: string;
	default_preset_id?: string;
}

export interface Project {
	id: string;
	name: string;
	description: string | null;
	created_at: string;
	updated_at: string;
	library_count: number;
	config?: ProjectConfig;
}

export interface Library {
	id: string;
	project_id: string;
	name: string;
	root_path: string;
	created_at: string;
	updated_at: string;
	last_indexed_at: string | null;
}

export interface CreateProjectRequest {
	name: string;
	description?: string;
	tags?: string[];
}

export interface CreateLibraryRequest {
	name: string;
	root_path: string;
}

export const projectsApi = {
	/**
	 * List all projects
	 */
	async list(): Promise<Project[]> {
		return apiClient.get<Project[]>('/api/projects');
	},

	/**
	 * Get a single project by ID
	 */
	async get(id: string): Promise<Project> {
		return apiClient.get<Project>(`/api/projects/${id}`);
	},

	/**
	 * Create a new project
	 */
	async create(data: CreateProjectRequest): Promise<Project> {
		return apiClient.post<Project>('/api/projects', data);
	},

	/**
	 * Delete a project
	 */
	async delete(id: string): Promise<void> {
		return apiClient.delete(`/api/projects/${id}`);
	},

	/**
	 * List libraries for a project
	 */
	async listLibraries(projectId: string): Promise<Library[]> {
		return apiClient.get<Library[]>(`/api/projects/${projectId}/libraries`);
	},

	/**
	 * Create a library for a project
	 */
	async createLibrary(projectId: string, data: CreateLibraryRequest): Promise<Library> {
		return apiClient.post<Library>(`/api/projects/${projectId}/libraries`, data);
	},

	/**
	 * Update project configuration (tools, models, workflows)
	 */
	async updateConfig(projectId: string, config: ProjectConfig): Promise<Project> {
		return apiClient.put<Project>(`/api/projects/${projectId}/config`, { config });
	}
};

export default projectsApi;
