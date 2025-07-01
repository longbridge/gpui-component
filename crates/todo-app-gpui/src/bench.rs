#[cfg(test)]
mod bench {
    use super::*;
    use std::time::Instant;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct BenchEvent {
        data: i32,
    }

    #[tokio::test]
    async fn bench_xbus_vs_ebus() {
        const ITERATIONS: usize = 100_000;

        // xbus 测试
        println!("Testing xbus...");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let start = Instant::now();
        {
            let _sub = crate::xbus::subscribe::<BenchEvent, _>(move |_event| {
                counter_clone.fetch_add(1, Ordering::Relaxed);
            });

            for i in 0..ITERATIONS {
                crate::xbus::post(BenchEvent { data: i as i32 });
            }
            
            // 等待所有事件处理完成
            while counter.load(Ordering::Relaxed) < ITERATIONS {
                std::thread::sleep(std::time::Duration::from_micros(1));
            }
        }
        let xbus_duration = start.elapsed();

        // ebus 测试  
        println!("Testing ebus...");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let start = Instant::now();
        {
            let mut receiver = crate::ebus::subscribe::<BenchEvent>();
            
            let handle = tokio::spawn(async move {
                for _i in 0..ITERATIONS {
                    if let Ok(_event) = receiver.recv().await {
                        counter_clone.fetch_add(1, Ordering::Relaxed);
                    }
                }
            });

            for i in 0..ITERATIONS {
                let _ = crate::ebus::post(BenchEvent { data: i as i32 });
            }
            
            // 等待所有事件处理完成
            handle.await.unwrap();
        }
        let ebus_duration = start.elapsed();

        println!("xbus: {:?} ({} events/sec)", 
                 xbus_duration, 
                 ITERATIONS * 1_000_000_000 / xbus_duration.as_nanos() as usize);
        println!("ebus: {:?} ({} events/sec)", 
                 ebus_duration,
                 ITERATIONS * 1_000_000_000 / ebus_duration.as_nanos() as usize);
        println!("xbus is {:.2}x faster", 
                 ebus_duration.as_nanos() as f64 / xbus_duration.as_nanos() as f64);
    }

    #[tokio::test]
    async fn bench_cross_runtime() {
        const ITERATIONS: usize = 10_000;
        
        println!("Testing cross-runtime performance...");
        
        // xbus 跨运行时测试
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let _sub = crate::xbus::subscribe::<BenchEvent, _>(move |_event| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });

        let start = Instant::now();
        
        // 从不同的运行时发送事件
        let handles: Vec<_> = (0..4).map(|thread_id| {
            let counter = counter.clone();
            tokio::spawn(async move {
                for i in 0..ITERATIONS/4 {
                    crate::xbus::post(BenchEvent { 
                        data: thread_id * 1000 + i as i32 
                    });
                }
            })
        }).collect();

        // 等待所有任务完成
        for handle in handles {
            handle.await.unwrap();
        }
        
        // 等待所有事件处理完成
        while counter.load(Ordering::Relaxed) < ITERATIONS {
            tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
        }
        
        let xbus_cross_duration = start.elapsed();
        
        // ebus 跨运行时测试
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        let mut receiver = crate::ebus::subscribe::<BenchEvent>();
        let recv_handle = tokio::spawn(async move {
            while counter_clone.load(Ordering::Relaxed) < ITERATIONS {
                if let Ok(_event) = receiver.recv().await {
                    counter_clone.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        let start = Instant::now();
        
        let handles: Vec<_> = (0..4).map(|thread_id| {
            tokio::spawn(async move {
                for i in 0..ITERATIONS/4 {
                    let _ = crate::ebus::post(BenchEvent { 
                        data: thread_id * 1000 + i as i32 
                    });
                }
            })
        }).collect();

        for handle in handles {
            handle.await.unwrap();
        }
        
        recv_handle.await.unwrap();
        let ebus_cross_duration = start.elapsed();

        println!("Cross-runtime xbus: {:?}", xbus_cross_duration);
        println!("Cross-runtime ebus: {:?}", ebus_cross_duration);
        println!("xbus is {:.2}x faster in cross-runtime scenarios", 
                 ebus_cross_duration.as_nanos() as f64 / xbus_cross_duration.as_nanos() as f64);
    }
}