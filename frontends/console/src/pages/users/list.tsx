import { List, useTable, TagField, TextField, EmailField } from "@refinedev/antd";
import { Table, Space, Button } from "antd";
import { EditOutlined } from "@ant-design/icons";
import type { BaseRecord } from "@refinedev/core";
import { useNavigation } from "@refinedev/core";

export const UserList = () => {
    const { tableProps } = useTable({
        syncWithLocation: true,
    });
    const { edit } = useNavigation();

    return (
        <List>
            <Table {...tableProps} rowKey="id">
                <Table.Column dataIndex="id" title="ID" />
                <Table.Column dataIndex="email" title="Email" render={(value) => <EmailField value={value} />} />
                <Table.Column dataIndex="display_name" title="Display Name" render={(value) => <TextField value={value} />} />
                <Table.Column 
                    dataIndex="global_role" 
                    title="Global Role" 
                    render={(value: string) => <TagField value={value} color={value === "super_admin" ? "volcano" : "blue"} />} 
                />
                <Table.Column 
                    dataIndex="status" 
                    title="Status" 
                    render={(value: string) => <TagField value={value} color={value === "active" ? "green" : "default"} />} 
                />
                <Table.Column
                    title="Actions"
                    dataIndex="actions"
                    render={(_, record: BaseRecord) => (
                        <Space>
                            <Button
                                size="small"
                                icon={<EditOutlined />}
                                onClick={() => edit("users", record.id as string)}
                            />
                        </Space>
                    )}
                />
            </Table>
        </List>
    );
};
