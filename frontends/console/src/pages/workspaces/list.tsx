import { List, useTable, TagField } from "@refinedev/antd";
import { Table, Space, Button } from "antd";
import { EditOutlined, EyeOutlined } from "@ant-design/icons";
import type { BaseRecord } from "@refinedev/core";
import { useNavigation } from "@refinedev/core";

export const WorkspaceList = () => {
    const { tableProps } = useTable({
        syncWithLocation: true,
    });
    const { edit, show } = useNavigation();

    return (
        <List>
            <Table {...tableProps} rowKey="id">
                <Table.Column dataIndex="id" title="ID" />
                <Table.Column dataIndex="name" title="Name" />
                <Table.Column dataIndex="slug" title="Slug" />
                <Table.Column 
                    dataIndex="status" 
                    title="Status" 
                    render={(value: string) => <TagField value={value} color={value === "active" ? "green" : "default"} />} 
                />
                <Table.Column 
                    dataIndex="role" 
                    title="Your Role" 
                    render={(value: string) => <TagField value={value} color="blue" />} 
                />
                <Table.Column
                    title="Actions"
                    dataIndex="actions"
                    render={(_, record: BaseRecord) => (
                        <Space>
                            <Button
                                size="small"
                                icon={<EyeOutlined />}
                                onClick={() => show("workspaces", record.id as string)}
                            />
                            <Button
                                size="small"
                                icon={<EditOutlined />}
                                onClick={() => edit("workspaces", record.id as string)}
                            />
                        </Space>
                    )}
                />
            </Table>
        </List>
    );
};
