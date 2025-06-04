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
        color: 0x93c5fd,
    },
    DataItem {
        month: "February",
        desktop: 305.,
        color: 0x60a5fa,
    },
    DataItem {
        month: "March",
        desktop: 237.,
        color: 0x3b82f6,
    },
    DataItem {
        month: "April",
        desktop: 73.,
        color: 0x2563eb,
    },
    DataItem {
        month: "May",
        desktop: 209.,
        color: 0x1d4ed8,
    },
    DataItem {
        month: "June",
        desktop: 214.,
        color: 0x1e40af,
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
        mobile: 111.,
    },
    DataItem2 {
        date: "Apr 2",
        desktop: 97.,
        mobile: 48.,
    },
    DataItem2 {
        date: "Apr 3",
        desktop: 167.,
        mobile: 84.,
    },
    DataItem2 {
        date: "Apr 4",
        desktop: 242.,
        mobile: 121.,
    },
    DataItem2 {
        date: "Apr 5",
        desktop: 373.,
        mobile: 187.,
    },
    DataItem2 {
        date: "Apr 6",
        desktop: 301.,
        mobile: 151.,
    },
    DataItem2 {
        date: "Apr 7",
        desktop: 245.,
        mobile: 123.,
    },
    DataItem2 {
        date: "Apr 8",
        desktop: 409.,
        mobile: 205.,
    },
    DataItem2 {
        date: "Apr 9",
        desktop: 59.,
        mobile: 30.,
    },
    DataItem2 {
        date: "Apr 10",
        desktop: 261.,
        mobile: 131.,
    },
    DataItem2 {
        date: "Apr 11",
        desktop: 327.,
        mobile: 164.,
    },
    DataItem2 {
        date: "Apr 12",
        desktop: 292.,
        mobile: 146.,
    },
    DataItem2 {
        date: "Apr 13",
        desktop: 342.,
        mobile: 171.,
    },
    DataItem2 {
        date: "Apr 14",
        desktop: 137.,
        mobile: 69.,
    },
    DataItem2 {
        date: "Apr 15",
        desktop: 120.,
        mobile: 60.,
    },
    DataItem2 {
        date: "Apr 16",
        desktop: 138.,
        mobile: 69.,
    },
    DataItem2 {
        date: "Apr 17",
        desktop: 446.,
        mobile: 223.,
    },
    DataItem2 {
        date: "Apr 18",
        desktop: 364.,
        mobile: 182.,
    },
    DataItem2 {
        date: "Apr 19",
        desktop: 243.,
        mobile: 122.,
    },
    DataItem2 {
        date: "Apr 20",
        desktop: 89.,
        mobile: 44.,
    },
    DataItem2 {
        date: "Apr 21",
        desktop: 137.,
        mobile: 69.,
    },
    DataItem2 {
        date: "Apr 22",
        desktop: 224.,
        mobile: 112.,
    },
    DataItem2 {
        date: "Apr 23",
        desktop: 138.,
        mobile: 69.,
    },
    DataItem2 {
        date: "Apr 24",
        desktop: 387.,
        mobile: 194.,
    },
    DataItem2 {
        date: "Apr 25",
        desktop: 215.,
        mobile: 108.,
    },
    DataItem2 {
        date: "Apr 26",
        desktop: 75.,
        mobile: 38.,
    },
    DataItem2 {
        date: "Apr 27",
        desktop: 383.,
        mobile: 192.,
    },
    DataItem2 {
        date: "Apr 28",
        desktop: 122.,
        mobile: 61.,
    },
    DataItem2 {
        date: "Apr 29",
        desktop: 315.,
        mobile: 158.,
    },
    DataItem2 {
        date: "Apr 30",
        desktop: 454.,
        mobile: 227.,
    },
    DataItem2 {
        date: "May 1",
        desktop: 165.,
        mobile: 82.,
    },
    DataItem2 {
        date: "May 2",
        desktop: 293.,
        mobile: 146.,
    },
    DataItem2 {
        date: "May 3",
        desktop: 247.,
        mobile: 124.,
    },
    DataItem2 {
        date: "May 4",
        desktop: 385.,
        mobile: 192.,
    },
    DataItem2 {
        date: "May 5",
        desktop: 481.,
        mobile: 241.,
    },
    DataItem2 {
        date: "May 6",
        desktop: 498.,
        mobile: 249.,
    },
    DataItem2 {
        date: "May 7",
        desktop: 388.,
        mobile: 194.,
    },
    DataItem2 {
        date: "May 8",
        desktop: 149.,
        mobile: 74.,
    },
    DataItem2 {
        date: "May 9",
        desktop: 227.,
        mobile: 114.,
    },
    DataItem2 {
        date: "May 10",
        desktop: 293.,
        mobile: 146.,
    },
    DataItem2 {
        date: "May 11",
        desktop: 335.,
        mobile: 168.,
    },
    DataItem2 {
        date: "May 12",
        desktop: 197.,
        mobile: 98.,
    },
    DataItem2 {
        date: "May 13",
        desktop: 197.,
        mobile: 98.,
    },
    DataItem2 {
        date: "May 14",
        desktop: 448.,
        mobile: 224.,
    },
    DataItem2 {
        date: "May 15",
        desktop: 473.,
        mobile: 236.,
    },
    DataItem2 {
        date: "May 16",
        desktop: 338.,
        mobile: 169.,
    },
    DataItem2 {
        date: "May 17",
        desktop: 499.,
        mobile: 250.,
    },
    DataItem2 {
        date: "May 18",
        desktop: 315.,
        mobile: 158.,
    },
    DataItem2 {
        date: "May 19",
        desktop: 235.,
        mobile: 118.,
    },
    DataItem2 {
        date: "May 20",
        desktop: 177.,
        mobile: 88.,
    },
    DataItem2 {
        date: "May 21",
        desktop: 82.,
        mobile: 41.,
    },
    DataItem2 {
        date: "May 22",
        desktop: 81.,
        mobile: 41.,
    },
    DataItem2 {
        date: "May 23",
        desktop: 252.,
        mobile: 126.,
    },
    DataItem2 {
        date: "May 24",
        desktop: 294.,
        mobile: 147.,
    },
    DataItem2 {
        date: "May 25",
        desktop: 201.,
        mobile: 100.,
    },
    DataItem2 {
        date: "May 26",
        desktop: 213.,
        mobile: 106.,
    },
    DataItem2 {
        date: "May 27",
        desktop: 420.,
        mobile: 210.,
    },
    DataItem2 {
        date: "May 28",
        desktop: 233.,
        mobile: 116.,
    },
    DataItem2 {
        date: "May 29",
        desktop: 78.,
        mobile: 39.,
    },
    DataItem2 {
        date: "May 30",
        desktop: 340.,
        mobile: 170.,
    },
    DataItem2 {
        date: "May 31",
        desktop: 178.,
        mobile: 89.,
    },
    DataItem2 {
        date: "Jun 1",
        desktop: 178.,
        mobile: 89.,
    },
    DataItem2 {
        date: "Jun 2",
        desktop: 470.,
        mobile: 235.,
    },
    DataItem2 {
        date: "Jun 3",
        desktop: 103.,
        mobile: 52.,
    },
    DataItem2 {
        date: "Jun 4",
        desktop: 439.,
        mobile: 220.,
    },
    DataItem2 {
        date: "Jun 5",
        desktop: 88.,
        mobile: 44.,
    },
    DataItem2 {
        date: "Jun 6",
        desktop: 294.,
        mobile: 147.,
    },
    DataItem2 {
        date: "Jun 7",
        desktop: 323.,
        mobile: 162.,
    },
    DataItem2 {
        date: "Jun 8",
        desktop: 385.,
        mobile: 192.,
    },
    DataItem2 {
        date: "Jun 9",
        desktop: 438.,
        mobile: 219.,
    },
    DataItem2 {
        date: "Jun 10",
        desktop: 155.,
        mobile: 78.,
    },
    DataItem2 {
        date: "Jun 11",
        desktop: 92.,
        mobile: 46.,
    },
    DataItem2 {
        date: "Jun 12",
        desktop: 492.,
        mobile: 246.,
    },
    DataItem2 {
        date: "Jun 13",
        desktop: 81.,
        mobile: 41.,
    },
    DataItem2 {
        date: "Jun 14",
        desktop: 426.,
        mobile: 213.,
    },
    DataItem2 {
        date: "Jun 15",
        desktop: 307.,
        mobile: 154.,
    },
    DataItem2 {
        date: "Jun 16",
        desktop: 371.,
        mobile: 186.,
    },
    DataItem2 {
        date: "Jun 17",
        desktop: 475.,
        mobile: 238.,
    },
    DataItem2 {
        date: "Jun 18",
        desktop: 107.,
        mobile: 54.,
    },
    DataItem2 {
        date: "Jun 19",
        desktop: 341.,
        mobile: 171.,
    },
    DataItem2 {
        date: "Jun 20",
        desktop: 408.,
        mobile: 204.,
    },
    DataItem2 {
        date: "Jun 21",
        desktop: 169.,
        mobile: 84.,
    },
    DataItem2 {
        date: "Jun 22",
        desktop: 317.,
        mobile: 158.,
    },
    DataItem2 {
        date: "Jun 23",
        desktop: 480.,
        mobile: 240.,
    },
    DataItem2 {
        date: "Jun 24",
        desktop: 132.,
        mobile: 66.,
    },
    DataItem2 {
        date: "Jun 25",
        desktop: 141.,
        mobile: 70.,
    },
    DataItem2 {
        date: "Jun 26",
        desktop: 434.,
        mobile: 217.,
    },
    DataItem2 {
        date: "Jun 27",
        desktop: 448.,
        mobile: 224.,
    },
    DataItem2 {
        date: "Jun 28",
        desktop: 149.,
        mobile: 74.,
    },
    DataItem2 {
        date: "Jun 29",
        desktop: 103.,
        mobile: 52.,
    },
    DataItem2 {
        date: "Jun 30",
        desktop: 446.,
        mobile: 223.,
    },
];
