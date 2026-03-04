-- 演示订单数据库
-- 包含客户、产品、订单、订单明细四张表

-- 客户表
CREATE TABLE IF NOT EXISTS customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    phone TEXT,
    address TEXT,
    city TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- 产品表
CREATE TABLE IF NOT EXISTS products (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('电子产品', '办公用品', '生活家居')),
    price REAL NOT NULL CHECK (price > 0),
    stock INTEGER NOT NULL DEFAULT 0 CHECK (stock >= 0),
    description TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now'))
);

-- 订单表
CREATE TABLE IF NOT EXISTS orders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('待付款', '已付款', '配送中', '已完成', '已取消')),
    total_amount REAL NOT NULL DEFAULT 0,
    remark TEXT,
    created_at DATETIME NOT NULL DEFAULT (datetime('now')),
    updated_at DATETIME NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (customer_id) REFERENCES customers(id)
);

-- 订单明细表
CREATE TABLE IF NOT EXISTS order_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    order_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    unit_price REAL NOT NULL CHECK (unit_price > 0),
    subtotal REAL GENERATED ALWAYS AS (quantity * unit_price) STORED,
    FOREIGN KEY (order_id) REFERENCES orders(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_orders_customer ON orders(customer_id);
CREATE INDEX IF NOT EXISTS idx_orders_status ON orders(status);
CREATE INDEX IF NOT EXISTS idx_order_items_order ON order_items(order_id);
CREATE INDEX IF NOT EXISTS idx_order_items_product ON order_items(product_id);
CREATE INDEX IF NOT EXISTS idx_products_category ON products(category);

-- ======== 示例数据 ========

-- 客户数据（10 条）
INSERT INTO customers (name, email, phone, address, city) VALUES
    ('张伟', 'zhangwei@example.com', '13800138001', '朝阳区建国路88号', '北京'),
    ('李娜', 'lina@example.com', '13900139002', '浦东新区陆家嘴环路100号', '上海'),
    ('王强', 'wangqiang@example.com', '13700137003', '天河区珠江新城华夏路', '广州'),
    ('刘洋', 'liuyang@example.com', '13600136004', '南山区科技园南路', '深圳'),
    ('陈静', 'chenjing@example.com', '13500135005', '武侯区天府大道北段', '成都'),
    ('赵磊', 'zhaolei@example.com', '13400134006', '江宁区秣陵街道', '南京'),
    ('孙芳', 'sunfang@example.com', '13300133007', '西湖区文三路', '杭州'),
    ('周明', 'zhouming@example.com', '13200132008', '岳麓区麓谷街道', '长沙'),
    ('吴秀英', 'wuxiuying@example.com', '13100131009', '高新区天府三街', '成都'),
    ('郑涛', 'zhengtao@example.com', '13000130010', '海淀区中关村南大街', '北京');

-- 产品数据（15 条，3 个类别）
INSERT INTO products (name, category, price, stock, description) VALUES
    ('机械键盘 K8 Pro', '电子产品', 599.00, 120, '87键蓝牙双模机械键盘，红轴'),
    ('无线鼠标 M720', '电子产品', 299.00, 200, '多设备切换无线鼠标，续航12个月'),
    ('USB-C 扩展坞', '电子产品', 459.00, 80, '8合1扩展坞，支持4K HDMI输出'),
    ('27寸4K显示器', '电子产品', 2499.00, 35, 'IPS面板，Type-C 65W反向充电'),
    ('降噪耳机 WH-900', '电子产品', 1299.00, 60, '主动降噪，30小时续航'),
    ('A4打印纸 (5包装)', '办公用品', 89.00, 500, '70g高白复印纸，500张/包'),
    ('中性笔套装', '办公用品', 25.00, 800, '0.5mm黑色中性笔，12支装'),
    ('文件收纳架', '办公用品', 45.00, 300, '三层桌面文件整理架，金属材质'),
    ('白板 (90x120cm)', '办公用品', 158.00, 50, '磁性白板，附赠白板笔和磁钉'),
    ('便利贴套装', '办公用品', 18.00, 1000, '76x76mm，5色各100张'),
    ('人体工学椅', '生活家居', 1599.00, 40, '可调节腰托和头枕，透气网面'),
    ('台灯 LED护眼', '生活家居', 199.00, 150, '无频闪，色温亮度可调'),
    ('加湿器', '生活家居', 129.00, 100, '超声波加湿，4L大容量水箱'),
    ('桌面收纳盒', '生活家居', 39.00, 400, '多格分类收纳，竹木材质'),
    ('咖啡杯保温套装', '生活家居', 69.00, 250, '316不锈钢内胆，500ml容量');

-- 订单数据（20 条，5 种状态）
INSERT INTO orders (customer_id, status, total_amount, remark, created_at, updated_at) VALUES
    (1, '已完成', 898.00, '请尽快发货', '2025-01-15 10:30:00', '2025-01-18 14:00:00'),
    (2, '已完成', 2499.00, NULL, '2025-01-20 09:15:00', '2025-01-25 16:30:00'),
    (1, '已付款', 459.00, '发票抬头：XX科技有限公司', '2025-02-01 11:00:00', '2025-02-01 11:05:00'),
    (3, '配送中', 1897.00, NULL, '2025-02-05 14:20:00', '2025-02-07 09:00:00'),
    (4, '待付款', 299.00, NULL, '2025-02-10 16:45:00', '2025-02-10 16:45:00'),
    (5, '已完成', 357.00, '送到前台即可', '2025-02-12 08:30:00', '2025-02-15 11:20:00'),
    (6, '已取消', 1599.00, '不需要了', '2025-02-14 13:00:00', '2025-02-14 15:30:00'),
    (2, '配送中', 623.00, NULL, '2025-02-18 10:10:00', '2025-02-20 08:45:00'),
    (7, '已付款', 2798.00, '周末送货', '2025-02-20 15:30:00', '2025-02-20 15:35:00'),
    (8, '已完成', 132.00, NULL, '2025-02-22 09:45:00', '2025-02-25 10:00:00'),
    (3, '待付款', 599.00, NULL, '2025-02-25 11:20:00', '2025-02-25 11:20:00'),
    (9, '已完成', 2957.00, '公司采购', '2025-02-28 14:00:00', '2025-03-03 09:30:00'),
    (10, '配送中', 757.00, NULL, '2025-03-01 10:00:00', '2025-03-03 14:00:00'),
    (4, '已付款', 1299.00, '生日礼物', '2025-03-05 16:30:00', '2025-03-05 16:35:00'),
    (5, '已完成', 89.00, NULL, '2025-03-08 08:00:00', '2025-03-10 12:00:00'),
    (1, '待付款', 1798.00, NULL, '2025-03-10 11:30:00', '2025-03-10 11:30:00'),
    (6, '已付款', 267.00, NULL, '2025-03-12 14:15:00', '2025-03-12 14:20:00'),
    (7, '已完成', 199.00, NULL, '2025-03-15 09:00:00', '2025-03-18 15:00:00'),
    (8, '配送中', 528.00, '请轻拿轻放', '2025-03-18 13:45:00', '2025-03-20 10:00:00'),
    (10, '已取消', 459.00, '买重复了', '2025-03-20 10:30:00', '2025-03-20 14:00:00');

-- 订单明细数据（33 条）
INSERT INTO order_items (order_id, product_id, quantity, unit_price) VALUES
    -- 订单1: 键盘 + 鼠标
    (1, 1, 1, 599.00),
    (1, 2, 1, 299.00),
    -- 订单2: 显示器
    (2, 4, 1, 2499.00),
    -- 订单3: 扩展坞
    (3, 3, 1, 459.00),
    -- 订单4: 键盘 + 耳机
    (4, 1, 1, 599.00),
    (4, 5, 1, 1299.00),
    -- 订单5: 鼠标
    (5, 2, 1, 299.00),
    -- 订单6: 打印纸 + 中性笔 + 收纳架
    (6, 6, 2, 89.00),
    (6, 7, 3, 25.00),
    (6, 8, 2, 45.00),
    -- 订单7: 人体工学椅
    (7, 11, 1, 1599.00),
    -- 订单8: 扩展坞 + 台灯
    (8, 3, 1, 459.00),
    (8, 12, 1, 199.00),
    -- 订单9: 显示器 + 鼠标
    (9, 4, 1, 2499.00),
    (9, 2, 1, 299.00),
    -- 订单10: 加湿器 + 收纳盒
    (10, 13, 1, 129.00),
    -- 订单11: 键盘
    (11, 1, 1, 599.00),
    -- 订单12: 显示器 + 扩展坞
    (12, 4, 1, 2499.00),
    (12, 3, 1, 459.00),
    -- 订单13: 键盘 + 白板
    (13, 1, 1, 599.00),
    (13, 9, 1, 158.00),
    -- 订单14: 耳机
    (14, 5, 1, 1299.00),
    -- 订单15: 打印纸
    (15, 6, 1, 89.00),
    -- 订单16: 人体工学椅 + 台灯
    (16, 11, 1, 1599.00),
    (16, 12, 1, 199.00),
    -- 订单17: 中性笔 + 便利贴 + 文件架
    (17, 7, 5, 25.00),
    (17, 10, 4, 18.00),
    (17, 8, 2, 45.00),
    -- 订单18: 台灯
    (18, 12, 1, 199.00),
    -- 订单19: 咖啡杯 + 扩展坞
    (19, 15, 1, 69.00),
    (19, 3, 1, 459.00),
    -- 订单20: 扩展坞
    (20, 3, 1, 459.00);
