import { List, useTable, TagField } from "@refinedev/antd";
import { Table, Space, Button, notification } from "antd";
import { ReloadOutlined } from "@ant-design/icons";
import { useCustomMutation } from "@refinedev/core";
import { useParams } from "react-router-dom";

export const CredentialList = () => {
    const { workspaceId } = useParams();
    const { tableProps } = useTable({
        syncWithLocation: true,
        resource: "credentials",
        meta: { workspaceId },
    });
    const { mutate } = useCustomMutation();

    const handleRotate = (keyId: string) => {
        mutate({
            url: `/workspaces/${workspaceId}/credentials/${keyId}/rotate`,
            method: "post",
            values: {},
        }, {
            onSuccess: (data) => {
                notification.success({
                    message: "Credential Rotated",
                    description: `New Secret: ${(data.data as any)?.api_secret || "Check console"}`,
                    duration: 0,
                });
            }
        });
    };

    return (
        <List>
            <Table {...tableProps} rowKey="api_key_id">
                <Table.Column dataIndex="api_key_id" title="API Key ID" />
                <Table.Column 
                    dataIndex="status" 
                    title="Status" 
                    render={(value: string) => <TagField value={value} color={value === "active" ? "green" : "red"} />} 
                />
                <Table.Column dataIndex="version" title="Version" />
                <Table.Column
                    title="Actions"
                    dataIndex="actions"
                    render={(_, record: any) => (
                        <Space>
                            <Button
                                size="small"
                                icon={<ReloadOutlined />}
                                onClick={() => handleRotate(record.api_key_id)}
                            >
                                Rotate
                            </Button>
                        </Space>
                    )}
                />
            </Table>
        </List>
    );
};
