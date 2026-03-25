import type { DataProvider } from "@refinedev/core";
import { axiosInstance } from "./axios";

export const dataProvider: DataProvider = {
  getList: async ({ resource, meta }) => {
    const url = meta?.workspaceId ? `/workspaces/${meta.workspaceId}/${resource}` : `/${resource}`;
    const response = await axiosInstance.get(url);
    const data = response.data;
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
