#[derive(Clone)]
pub struct DataItem {
    pub month: &'static str,
    pub desktop: f64,
    pub color: u32,
}

pub const CHART_DATA: &[DataItem] = &[
    DataItem {
        month: "January",
        desktop: 186.,
        color: 0x2a9d90,
    },
    DataItem {
        month: "February",
        desktop: 305.,
        color: 0xe76e50,
    },
    DataItem {
        month: "March",
        desktop: 237.,
        color: 0x274754,
    },
    DataItem {
        month: "April",
        desktop: 73.,
        color: 0xe8c468,
    },
    DataItem {
        month: "May",
        desktop: 209.,
        color: 0xf4a462,
    },
    DataItem {
        month: "June",
        desktop: 214.,
        color: 0x2563eb,
    },
];

#[derive(Clone)]
pub struct DataItem2 {
    pub date: &'static str,
    pub desktop: f64,
    #[allow(dead_code)]
    pub mobile: f64,
}

pub const CHART_DATA_2: &[DataItem2] = &[
    DataItem2 {
        date: "Apr 1",
        desktop: 222.,
        mobile: 150.,
    },
    DataItem2 {
        date: "Apr 2",
        desktop: 97.,
        mobile: 180.,
    },
    DataItem2 {
        date: "Apr 3",
        desktop: 167.,
        mobile: 120.,
    },
    DataItem2 {
        date: "Apr 4",
        desktop: 242.,
        mobile: 260.,
    },
    DataItem2 {
        date: "Apr 5",
        desktop: 373.,
        mobile: 290.,
    },
    DataItem2 {
        date: "Apr 6",
        desktop: 301.,
        mobile: 340.,
    },
    DataItem2 {
        date: "Apr 7",
        desktop: 245.,
        mobile: 180.,
    },
    DataItem2 {
        date: "Apr 8",
        desktop: 409.,
        mobile: 320.,
    },
    DataItem2 {
        date: "Apr 9",
        desktop: 59.,
        mobile: 110.,
    },
    DataItem2 {
        date: "Apr 10",
        desktop: 261.,
        mobile: 190.,
    },
    DataItem2 {
        date: "Apr 11",
        desktop: 327.,
        mobile: 350.,
    },
    DataItem2 {
        date: "Apr 12",
        desktop: 292.,
        mobile: 210.,
    },
    DataItem2 {
        date: "Apr 13",
        desktop: 342.,
        mobile: 380.,
    },
    DataItem2 {
        date: "Apr 14",
        desktop: 137.,
        mobile: 220.,
    },
    DataItem2 {
        date: "Apr 15",
        desktop: 120.,
        mobile: 170.,
    },
    DataItem2 {
        date: "Apr 16",
        desktop: 138.,
        mobile: 190.,
    },
    DataItem2 {
        date: "Apr 17",
        desktop: 446.,
        mobile: 360.,
    },
    DataItem2 {
        date: "Apr 18",
        desktop: 364.,
        mobile: 410.,
    },
    DataItem2 {
        date: "Apr 19",
        desktop: 243.,
        mobile: 180.,
    },
    DataItem2 {
        date: "Apr 20",
        desktop: 89.,
        mobile: 150.,
    },
    DataItem2 {
        date: "Apr 21",
        desktop: 137.,
        mobile: 200.,
    },
    DataItem2 {
        date: "Apr 22",
        desktop: 224.,
        mobile: 170.,
    },
    DataItem2 {
        date: "Apr 23",
        desktop: 138.,
        mobile: 230.,
    },
    DataItem2 {
        date: "Apr 24",
        desktop: 387.,
        mobile: 290.,
    },
    DataItem2 {
        date: "Apr 25",
        desktop: 215.,
        mobile: 250.,
    },
    DataItem2 {
        date: "Apr 26",
        desktop: 75.,
        mobile: 130.,
    },
    DataItem2 {
        date: "Apr 27",
        desktop: 383.,
        mobile: 420.,
    },
    DataItem2 {
        date: "Apr 28",
        desktop: 122.,
        mobile: 180.,
    },
    DataItem2 {
        date: "Apr 29",
        desktop: 315.,
        mobile: 240.,
    },
    DataItem2 {
        date: "Apr 30",
        desktop: 454.,
        mobile: 380.,
    },
    DataItem2 {
        date: "May 1",
        desktop: 165.,
        mobile: 220.,
    },
    DataItem2 {
        date: "May 2",
        desktop: 293.,
        mobile: 310.,
    },
    DataItem2 {
        date: "May 3",
        desktop: 247.,
        mobile: 190.,
    },
    DataItem2 {
        date: "May 4",
        desktop: 385.,
        mobile: 420.,
    },
    DataItem2 {
        date: "May 5",
        desktop: 481.,
        mobile: 390.,
    },
    DataItem2 {
        date: "May 6",
        desktop: 498.,
        mobile: 520.,
    },
    DataItem2 {
        date: "May 7",
        desktop: 388.,
        mobile: 300.,
    },
    DataItem2 {
        date: "May 8",
        desktop: 149.,
        mobile: 210.,
    },
    DataItem2 {
        date: "May 9",
        desktop: 227.,
        mobile: 180.,
    },
    DataItem2 {
        date: "May 10",
        desktop: 293.,
        mobile: 330.,
    },
    DataItem2 {
        date: "May 11",
        desktop: 335.,
        mobile: 270.,
    },
    DataItem2 {
        date: "May 12",
        desktop: 197.,
        mobile: 240.,
    },
    DataItem2 {
        date: "May 13",
        desktop: 197.,
        mobile: 160.,
    },
    DataItem2 {
        date: "May 14",
        desktop: 448.,
        mobile: 490.,
    },
    DataItem2 {
        date: "May 15",
        desktop: 473.,
        mobile: 380.,
    },
    DataItem2 {
        date: "May 16",
        desktop: 338.,
        mobile: 400.,
    },
    DataItem2 {
        date: "May 17",
        desktop: 499.,
        mobile: 420.,
    },
    DataItem2 {
        date: "May 18",
        desktop: 315.,
        mobile: 350.,
    },
    DataItem2 {
        date: "May 19",
        desktop: 235.,
        mobile: 180.,
    },
    DataItem2 {
        date: "May 20",
        desktop: 177.,
        mobile: 230.,
    },
    DataItem2 {
        date: "May 21",
        desktop: 82.,
        mobile: 140.,
    },
    DataItem2 {
        date: "May 22",
        desktop: 81.,
        mobile: 120.,
    },
    DataItem2 {
        date: "May 23",
        desktop: 252.,
        mobile: 290.,
    },
    DataItem2 {
        date: "May 24",
        desktop: 294.,
        mobile: 220.,
    },
    DataItem2 {
        date: "May 25",
        desktop: 201.,
        mobile: 250.,
    },
    DataItem2 {
        date: "May 26",
        desktop: 213.,
        mobile: 170.,
    },
    DataItem2 {
        date: "May 27",
        desktop: 420.,
        mobile: 460.,
    },
    DataItem2 {
        date: "May 28",
        desktop: 233.,
        mobile: 190.,
    },
    DataItem2 {
        date: "May 29",
        desktop: 78.,
        mobile: 130.,
    },
    DataItem2 {
        date: "May 30",
        desktop: 340.,
        mobile: 280.,
    },
    DataItem2 {
        date: "May 31",
        desktop: 178.,
        mobile: 230.,
    },
    DataItem2 {
        date: "Jun 1",
        desktop: 178.,
        mobile: 200.,
    },
    DataItem2 {
        date: "Jun 2",
        desktop: 470.,
        mobile: 410.,
    },
    DataItem2 {
        date: "Jun 3",
        desktop: 103.,
        mobile: 160.,
    },
    DataItem2 {
        date: "Jun 4",
        desktop: 439.,
        mobile: 380.,
    },
    DataItem2 {
        date: "Jun 5",
        desktop: 88.,
        mobile: 140.,
    },
    DataItem2 {
        date: "Jun 6",
        desktop: 294.,
        mobile: 250.,
    },
    DataItem2 {
        date: "Jun 7",
        desktop: 323.,
        mobile: 370.,
    },
    DataItem2 {
        date: "Jun 8",
        desktop: 385.,
        mobile: 320.,
    },
    DataItem2 {
        date: "Jun 9",
        desktop: 438.,
        mobile: 480.,
    },
    DataItem2 {
        date: "Jun 10",
        desktop: 155.,
        mobile: 200.,
    },
    DataItem2 {
        date: "Jun 11",
        desktop: 92.,
        mobile: 150.,
    },
    DataItem2 {
        date: "Jun 12",
        desktop: 492.,
        mobile: 420.,
    },
    DataItem2 {
        date: "Jun 13",
        desktop: 81.,
        mobile: 130.,
    },
    DataItem2 {
        date: "Jun 14",
        desktop: 426.,
        mobile: 380.,
    },
    DataItem2 {
        date: "Jun 15",
        desktop: 307.,
        mobile: 350.,
    },
    DataItem2 {
        date: "Jun 16",
        desktop: 371.,
        mobile: 310.,
    },
    DataItem2 {
        date: "Jun 17",
        desktop: 475.,
        mobile: 520.,
    },
    DataItem2 {
        date: "Jun 18",
        desktop: 107.,
        mobile: 170.,
    },
    DataItem2 {
        date: "Jun 19",
        desktop: 341.,
        mobile: 290.,
    },
    DataItem2 {
        date: "Jun 20",
        desktop: 408.,
        mobile: 450.,
    },
    DataItem2 {
        date: "Jun 21",
        desktop: 169.,
        mobile: 210.,
    },
    DataItem2 {
        date: "Jun 22",
        desktop: 317.,
        mobile: 270.,
    },
    DataItem2 {
        date: "Jun 23",
        desktop: 480.,
        mobile: 530.,
    },
    DataItem2 {
        date: "Jun 24",
        desktop: 132.,
        mobile: 180.,
    },
    DataItem2 {
        date: "Jun 25",
        desktop: 141.,
        mobile: 190.,
    },
    DataItem2 {
        date: "Jun 26",
        desktop: 434.,
        mobile: 380.,
    },
    DataItem2 {
        date: "Jun 27",
        desktop: 448.,
        mobile: 490.,
    },
    DataItem2 {
        date: "Jun 28",
        desktop: 149.,
        mobile: 200.,
    },
    DataItem2 {
        date: "Jun 29",
        desktop: 103.,
        mobile: 160.,
    },
    DataItem2 {
        date: "Jun 30",
        desktop: 446.,
        mobile: 400.,
    },
];
