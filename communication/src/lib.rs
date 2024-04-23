pub mod protos;

pub trait TeamLike {
    fn unique_id(&self) -> i32;
}

pub type ProtobufAvailabilityWindow = (i64, i64);

pub trait FieldLike {
    fn unique_id(&self) -> i32;
    fn time_slots(&self) -> impl AsRef<[(ProtobufAvailabilityWindow, u8)]>;
}

impl TeamLike for protos::algo_input::Team {
    fn unique_id(&self) -> i32 {
        self.unique_id.try_into().expect("team id is too big")
    }
}

impl FieldLike for protos::algo_input::Field {
    fn unique_id(&self) -> i32 {
        self.unique_id.try_into().expect("field id is too big")
    }

    fn time_slots(&self) -> impl AsRef<[(ProtobufAvailabilityWindow, u8)]> {
        self.time_slots
            .iter()
            .map(|time_slot| {
                let availability = (time_slot.start, time_slot.end);

                (
                    availability,
                    time_slot
                        .concurrency
                        .try_into()
                        .expect("concurrency is too big"),
                )
            })
            .collect::<Vec<_>>()
    }
}
