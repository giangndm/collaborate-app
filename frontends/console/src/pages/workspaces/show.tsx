import { Show, TextField, DateField } from "@refinedev/antd";
import { Typography, Space, Button } from "antd";
import { useShow } from "@refinedev/core";
import { Link } from "react-router-dom";

export const WorkspaceShow = () => {
    const { queryResult } = useShow();
    const { data, isLoading } = queryResult;
    const record = data?.data;

    return (
        <Show isLoading={isLoading}>
            <Space style={{ marginBottom: 16 }}>
                <Link to={`/workspaces/${record?.id}/members`}>
                    <Button type="primary">Manage Members</Button>
                </Link>
                <Link to={`/workspaces/${record?.id}/credentials`}>
                    <Button>Manage Credentials</Button>
                </Link>
            </Space>

            <Typography.Title level={5}>ID</Typography.Title>
            <TextField value={record?.id} />

            <Typography.Title level={5}>Name</Typography.Title>
            <TextField value={record?.name} />

            <Typography.Title level={5}>Slug</Typography.Title>
            <TextField value={record?.slug} />

            <Typography.Title level={5}>Status</Typography.Title>
            <TextField value={record?.status} />

            <Typography.Title level={5}>Created At</Typography.Title>
            <DateField value={record?.created_at} format="YYYY-MM-DD HH:mm:ss" />
        </Show>
    );
};
