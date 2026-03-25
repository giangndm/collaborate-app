import type { AuthProvider } from "@refinedev/core";
import { axiosInstance } from "./axios";

export const authProvider: AuthProvider = {
  login: async ({ email, password }) => {
    try {
      await axiosInstance.post("/auth/login", { email, password });
      localStorage.setItem("authenticated", "true");
      return {
        success: true,
        redirectTo: "/",
      };
    } catch (e: any) {
      return {
        success: false,
        error: {
          name: "LoginError",
          message: e.response?.data?.message || "Login failed",
        },
      };
    }
  },
  logout: async () => {
    try {
      await axiosInstance.post("/auth/logout");
    } catch (e) {
      // ignore
    }
    localStorage.removeItem("authenticated");
    return {
      success: true,
      redirectTo: "/login",
    };
  },
  check: async () => {
    const isAuth = localStorage.getItem("authenticated");
    if (isAuth === "true") {
      return {
        authenticated: true,
      };
    }
    return {
      authenticated: false,
      redirectTo: "/login",
      logout: true,
    };
  },
  getPermissions: async () => null,
  getIdentity: async () => {
    if (localStorage.getItem("authenticated") === "true") {
      try {
        const response = await axiosInstance.get("/auth/me");
        return response.data;
      } catch (error) {
        return null;
      }
    }
    return null;
  },
  onError: async (error) => {
    if (error.response?.status === 401 || error.response?.status === 403) {
      return {
        logout: true,
      };
    }
    return { error };
  },
};
