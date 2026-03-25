import type { DataProvider } from "@refinedev/core";
import { axiosInstance } from "./axios";

export const dataProvider: DataProvider = {
  getList: async ({ resource, meta, filters }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}` : `/${resource}`;
    
    // Convert Refine filters to query params
    const params: Record<string, any> = {};
    if (filters) {
      filters.forEach((filter) => {
        if ("field" in filter && "value" in filter) {
          params[filter.field] = filter.value;
        }
      });
    }

    const response = await axiosInstance.get(url, { params });
    const data = response.data;
    
    // Handle both direct array and RefineListResponse { data, total }
    if (data && typeof data === 'object' && 'data' in data) {
      return {
        data: data.data,
        total: data.total ?? data.data.length,
      };
    }

    return {
      data,
      total: data.length,
    };
  },
  getOne: async ({ resource, id, meta }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}/${id}` : `/${resource}/${id}`;
    const response = await axiosInstance.get(url);
    return {
      data: response.data,
    };
  },
  create: async ({ resource, variables, meta }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}` : `/${resource}`;
    const response = await axiosInstance.post(url, variables);
    return {
      data: response.data,
    };
  },
  update: async ({ resource, id, variables, meta }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}/${id}` : `/${resource}/${id}`;
    const response = await axiosInstance.put(url, variables);
    return {
      data: response.data,
    };
  },
  deleteOne: async ({ resource, id, meta }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}/${id}` : `/${resource}/${id}`;
    const response = await axiosInstance.delete(url);
    return {
      data: response.data,
    };
  },
  getApiUrl: () => "http://localhost:3000",
  custom: async ({ url, method, payload, query }) => {
    const requestUrl = `${url}`;

    const response = await axiosInstance.request({
      url: requestUrl,
      method,
      data: payload,
      params: query,
    });
    return { data: response.data };
  },
};
