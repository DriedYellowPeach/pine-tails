use crate::utils::TestApp;
use pine_tails::routes::playground::RecognizeInfo;

const IMG_TWO: &str = "\
............................
............................
.........xxxxxx.............
.......xxxxxxxxx............
......xxxx....xxx...........
......xx.......xx...........
......xx.......xx...........
......xx.......xx...........
.......x......xx............
..............xx............
.............xxx............
............xxx.............
...........xxx..............
..........xxxx..............
..........xxx...............
.........xxx................
........xxx.................
......xxxx......xxxx........
.....xxxxxxxxxxxxxxxx.......
....xxxxxxxxxxxxxxxxx.......
....xxxxxx..................
....xxx.....................
............................
............................
............................
............................
............................
............................";

#[tokio::test]
async fn recognize_digit_with_pixel_vector_vector_returns_ok() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/playground/digit_recognition", app.address);

    let img: Vec<u8> = IMG_TWO
        .lines()
        .flat_map(|line| {
            line.chars()
                .map(|c| if c == '.' { 0 } else { 255 })
                .collect::<Vec<u8>>()
        })
        .collect();

    assert_eq!(img.len(), 784);

    let response = app
        .client
        .post(api_addr)
        .json(&serde_json::json!(
            {
                "Vector": img,
            }
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);
    let rec_res = response.json::<RecognizeInfo>().await.unwrap();
    assert_eq!(rec_res.digit, 2);
    assert!(rec_res.confidence > 0.5);
}

#[tokio::test]
async fn recognize_digit_with_base64_str_returns_ok() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/playground/digit_recognition", app.address);

    let img_three = "iVBORw0KGgoAAAANSUhEUgAAABwAAAAcCAYAAAByDd+UAAABfUlEQVRIS8WWoU4DQRCGu6GEhCBQaFAIJKpP0FSR1FSDIWB4ARwvgANEi69pZcMTIKoRGLAoBAICJMd3F+6yuezOTMOxbbJJ25n7v/lnd6d1rcQvl5jXMgGzLJtS2EGguFvn3NEiRatAYJkmCFTVKTXERB8WEvXi78TXtcLyeBSI2AbxtyJJcFBCrS41h8cI3UiVA/wmvtII0NIigBfknbNOgV5pz5g3OyYE8IPYGmsH4HMKYHGKk7QUd2ewLlknAK81d+IprT+M+CvfbYZEre5MQEBbJL5o1Vuh6qERJk0fyIR4PtqG1SRRpo52Dz8RWq2566B5H2h5NQIlt6pDrZV+HLdzPu+zekBnwf1eRNCSq426Rh3mBS0LeEdLu//eUs1d9B7+PvhAlXuWfStz/go0z0fL3lX3NOSASp/4frtogeHvA/lfpLYt+dIvvv9fZhfuY6Q404UXHXp7MuL9oQfKxSesfh1u6UT00EhjK+ByDGxgPVyNX3wNnBz4A9bykh1vbHb/AAAAAElFTkSuQmCC";
    let response = app
        .client
        .post(api_addr)
        .json(&serde_json::json!(
            {
                "Base64": img_three,
            }
        ))
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 200);

    let rec_res = response.json::<RecognizeInfo>().await.unwrap();
    assert_eq!(rec_res.digit, 3);
}

#[tokio::test]
async fn recognize_digit_with_small_pixel_vector_returns_bad_request() {
    let app = TestApp::spawn_server().await;
    let api_addr = format!("{}/playground/digit_recognition", app.address);
    let img = vec![0; 10];
    let response = app
        .client
        .post(api_addr)
        .json(&img)
        .send()
        .await
        .expect("Failed to send request");

    assert_eq!(response.status().as_u16(), 400);
}
