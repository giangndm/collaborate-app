import { List, useTable, TagField } from "@refinedev/antd";
import { Table, Space, Button } from "antd";
import { DeleteOutlined } from "@ant-design/icons";
import { useDelete } from "@refinedev/core";
import { useParams } from "react-router-dom";

export const MemberList = () => {
    const { workspaceId } = useParams();
    const { tableProps } = useTable({
        syncWithLocation: true,
        resource: "members",
        meta: { workspaceId },
    });
    const { mutate } = useDelete();

    return (
        <List>
            <Table {...tableProps} rowKey="user_id">
                <Table.Column dataIndex="display_name" title="Name" />
                <Table.Column dataIndex="email" title="Email" />
                <Table.Column dataIndex="role" title="Role" />
                <Table.Column 
                    dataIndex="status" 
                    title="Status" 
                    render={(value: string) => <TagField value={value} color={value === "accepted" ? "green" : "default"} />} 
                />
                <Table.Column
                    title="Actions"
                    dataIndex="actions"
                    render={(_, record: any) => (
                        <Space>
                            <Button
                                size="small"
                                danger
                                icon={<DeleteOutlined />}
                                onClick={() => mutate({ resource: "members", id: record.user_id, meta: { workspaceId } })}
                            />
                        </Space>
                    )}
                />
            </Table>
        </List>
    );
};
