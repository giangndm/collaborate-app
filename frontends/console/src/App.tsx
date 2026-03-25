import { Refine, Authenticated } from "@refinedev/core";
import { ConfigProvider, App as AntdApp } from "antd";
import { BrowserRouter, Route, Routes, Outlet } from "react-router-dom";
import routerBindings, { NavigateToResource, CatchAllNavigate, UnsavedChangesNotifier, DocumentTitleHandler } from "@refinedev/react-router-v6";
import { authProvider } from "./providers/authProvider";
import { dataProvider } from "./providers/dataProvider";
import { ThemedLayoutV2, ErrorComponent, AuthPage } from "@refinedev/antd";
import { WorkspaceList, WorkspaceCreate, WorkspaceEdit, WorkspaceShow } from "./pages/workspaces";
import { MemberList, MemberCreate } from "./pages/members";
import { CredentialList, CredentialCreate } from "./pages/credentials";
import "@refinedev/antd/dist/reset.css";

function App() {
  return (
    <BrowserRouter>
      <ConfigProvider>
        <AntdApp>
          <Refine
            routerProvider={routerBindings}
            authProvider={authProvider}
            dataProvider={dataProvider}
            resources={[
              {
                name: "workspaces",
                list: "/workspaces",
                create: "/workspaces/create",
                edit: "/workspaces/edit/:id",
                show: "/workspaces/show/:id",
                meta: {
                  canDelete: true,
                },
              },
              {
                name: "members",
                list: "/workspaces/:workspaceId/members",
                create: "/workspaces/:workspaceId/members/create",
                meta: {
                  parent: "workspaces",
                  canDelete: true,
                  hide: true,
                },
              },
              {
                name: "credentials",
                list: "/workspaces/:workspaceId/credentials",
                create: "/workspaces/:workspaceId/credentials/create",
                meta: {
                  parent: "workspaces",
                  hide: true,
                },
              },
            ]}
            options={{
              syncWithLocation: true,
              warnWhenUnsavedChanges: true,
            }}
          >
            <Routes>
              <Route
                element={
                  <Authenticated
                    key="authenticated-inner"
                    fallback={<CatchAllNavigate to="/login" />}
                  >
                    <ThemedLayoutV2>
                      <Outlet />
                    </ThemedLayoutV2>
                  </Authenticated>
                }
              >
                <Route index element={<NavigateToResource resource="workspaces" />} />
                <Route path="/workspaces">
                  <Route index element={<WorkspaceList />} />
                  <Route path="create" element={<WorkspaceCreate />} />
                  <Route path="edit/:id" element={<WorkspaceEdit />} />
                  <Route path="show/:id" element={<WorkspaceShow />} />
                  <Route path=":workspaceId/members">
                    <Route index element={<MemberList />} />
                    <Route path="create" element={<MemberCreate />} />
                  </Route>
                  <Route path=":workspaceId/credentials">
                    <Route index element={<CredentialList />} />
                    <Route path="create" element={<CredentialCreate />} />
                  </Route>
                </Route>
              </Route>
              
              <Route
                element={
                  <Authenticated
                    key="authenticated-outer"
                    fallback={<Outlet />}
                  >
                    <NavigateToResource />
                  </Authenticated>
                }
              >
                <Route path="/login" element={<AuthPage type="login" />} />
              </Route>

              <Route path="*" element={<ErrorComponent />} />
            </Routes>
            <UnsavedChangesNotifier />
            <DocumentTitleHandler />
          </Refine>
        </AntdApp>
      </ConfigProvider>
    </BrowserRouter>
  );
}

export default App;
