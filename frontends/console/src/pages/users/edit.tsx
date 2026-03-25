import { Edit, useForm } from "@refinedev/antd";
import { Form, Input, Select } from "antd";

export const UserEdit = () => {
    const { formProps, saveButtonProps, queryResult } = useForm();

    return (
        <Edit saveButtonProps={saveButtonProps}>
            <Form {...formProps} layout="vertical">
                <Form.Item
                    label="Email"
                    name="email"
                >
                    <Input disabled />
                </Form.Item>
                <Form.Item
                    label="Display Name"
                    name="display_name"
                    rules={[{ required: true }]}
                >
                    <Input />
                </Form.Item>
                <Form.Item
                    label="Global Role"
                    name="global_role"
                    rules={[{ required: true }]}
                >
                    <Select>
                        <Select.Option value="member">Member</Select.Option>
                        <Select.Option value="super_admin">Super Admin</Select.Option>
                    </Select>
                </Form.Item>
                <Form.Item
                    label="Status"
                    name="status"
                    rules={[{ required: true }]}
                >
                    <Select>
                        <Select.Option value="active">Active</Select.Option>
                        <Select.Option value="suspended">Suspended</Select.Option>
                        <Select.Option value="disabled">Disabled</Select.Option>
                    </Select>
                </Form.Item>
            </Form>
        </Edit>
    );
};
